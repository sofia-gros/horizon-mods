#!/usr/bin/env python3
"""Local smoke gate: launch the built, modded Horizon client and assert one thing.

Not run in CI (needs a built client and a prepared Horizon 26.3 resource folder).
Prints `SMOKE PASS` only when every step below really ran; otherwise it exits
non-zero and says which step failed. Nothing here claims more than it executed.

Steps: (1) pin check, (2) launch offscreen into a fresh test world, (3) wait for
the control endpoint and world readiness (= game started, mod hooks loaded),
(4) two assertions: in survival, the mod's data recipes turn 1 flint + 4 sticks into the
three TConstruct parts and then a flint pickaxe (inventory crafting grid); and the real Part Builder
block opens a container screen, (5) clean quit.
"""
import argparse, json, os, subprocess, sys, tempfile, time
from pathlib import Path

root = Path(__file__).resolve().parents[2]
p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True, help='horizon-client executable (modded build)')
p.add_argument('--ctl', type=Path, required=True, help='horizon-ctl executable')
p.add_argument('--resources', type=Path, required=True, help='prepared Horizon 26.3 resource folder')
p.add_argument('--run-dir', type=Path, default=root / '.local/smoke-runs')
p.add_argument('--screenshots', type=Path, help='also save in-game screenshots of the checked steps into this folder')
a = p.parse_args()

compat = json.loads((root / 'tconstruct/compatibility.json').read_text())
revfile = root / 'tconstruct/target/mixins-revision.txt'
staged = (root / 'tconstruct/target/original/.horizon-revision')
if staged.is_file() and staged.read_text().strip() != compat['horizon_revision']:
    sys.exit('SMOKE FAIL (pin): staged Horizon revision differs from tconstruct/compatibility.json')
if revfile.is_file() and revfile.read_text().strip() != compat['mixins_revision']:
    sys.exit('SMOKE FAIL (pin): staged Mixins revision differs from tconstruct/compatibility.json')
if not (a.resources / 'horizon-resources.json').is_file():
    sys.exit('SMOKE FAIL (setup): --resources is not a prepared Horizon resource folder')

a.run_dir.mkdir(parents=True, exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='smoke-', dir=a.run_dir))
endpoint = run / 'control.json'
env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off', RUST_MIN_STACK='16777216')
for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
    folder = run / key.lower(); folder.mkdir(); env[key] = str(folder)
cmd = [str(a.game.resolve()), '--resources', str(a.resources.resolve()), '--control-socket', str(endpoint), '--offscreen',
       '--world', 'Probe', '--seed', '42', '--creative', '--test-terrain', '--render-distance', '2', '--width', *(('1280', '--height', '720') if a.screenshots else ('640', '--height', '360'))]
log = (run / 'game.log').open('w')
proc = subprocess.Popen(cmd, cwd=run, env=env, stdout=log, stderr=subprocess.STDOUT)

def ctl(*args, timeout=30):
    r = subprocess.run([str(a.ctl.resolve()), '--socket', str(endpoint), '--timeout', str(timeout), *args], capture_output=True, text=True, timeout=timeout + 5)
    if r.returncode:
        raise RuntimeError(f'ctl {args}: {r.stderr.strip() or r.stdout.strip()}')
    value = json.loads(r.stdout)
    if value.get('error'):
        raise RuntimeError(str(value))
    return value

def shot(name):
    if a.screenshots:
        a.screenshots.mkdir(parents=True, exist_ok=True)
        time.sleep(.7)
        ctl('screenshot', str((a.screenshots / name).resolve()))

step = 'launch'
try:
    deadline = time.monotonic() + 120
    while not endpoint.exists():
        if proc.poll() is not None: raise RuntimeError(f'game exited early (code {proc.returncode})')
        if time.monotonic() > deadline: raise TimeoutError('control endpoint never appeared')
        time.sleep(.3)
    step = 'world ready'
    ctl('wait-ready', '120', timeout=125)
    step = 'assertion: craft flint pickaxe from parts'
    # Data recipes only (no mod command): flint -> pick head, 2 sticks (vertical) -> handle,
    # 2 sticks (horizontal) -> binding, then the three parts -> flint pickaxe.
    for c in ('clear @s', 'gamemode survival @s', 'give @s minecraft:flint 1', 'give @s minecraft:stick 4'):
        ctl('command', c)
    time.sleep(1)
    held = ctl('inventory')
    hotbar = {s['item']: i for i, s in enumerate(held['hotbar']) if s}
    flint_slot, stick_slot = hotbar['minecraft:flint'], hotbar['minecraft:stick']
    ctl('open-inventory'); time.sleep(.5)
    def click(*a): ctl('click-slot', *map(str, a)); time.sleep(.2)
    def craft(into):                      # take the result, store it in an inventory slot
        click(200); click(into)
    click(flint_slot); click(100); craft(9)                 # pick head
    click(stick_slot); click(100, 'right'); click(102, 'right'); click(stick_slot); craft(10)   # handle (vertical); put the other 2 sticks back first
    click(stick_slot); click(100, 'right'); click(101, 'right'); craft(11)     # binding (horizontal)
    click(9); click(100); click(10); click(101); click(11); click(102)  # the three parts
    craft(12)                                                # flint pickaxe
    shot('crafted-flint-pickaxe.png')
    ctl('close'); time.sleep(.5)
    counts = ctl('inventory')['counts']
    assert counts.get('tconstruct:flint_pickaxe') == 1, counts
    assert not any(counts.get(x) for x in ('tconstruct:flint_pick_head', 'tconstruct:wood_binding', 'tconstruct:wood_handle')), counts
    step = 'assertion: Part Builder opens'
    # The real Part Builder block (mod hooks): placing it and right-clicking must open a container screen with slots.
    ctl('command', 'setblock 0 71 2 tconstruct:part_builder'); ctl('command', 'tp @s 0.5 71 0.5'); time.sleep(2.5 if a.screenshots else 1)
    ctl('look-at-block', '0', '71', '2'); time.sleep(.5); opened = ctl('interact', '0', '71', '2'); time.sleep(2)
    screen = ctl('screen')
    assert opened.get('screen') == 'container' and screen['screen'] == 'container', (opened.get('screen'), screen['screen'])
    assert len([x for x in screen['slots'] if x['slot'] >= 300]) >= 9, 'Part Builder container has no station slots'
    shot('part-builder.png')
    ctl('close'); time.sleep(.5)
    step = 'quit'
    ctl('quit'); proc.wait(timeout=60)
    assert proc.returncode == 0, f'exit code {proc.returncode}'
    print(json.dumps({'result': 'SMOKE PASS', 'run': str(run), 'inventory': counts}, indent=2))
except BaseException as error:
    print(f'SMOKE FAIL at step "{step}": {error!r}\nlog: {run / "game.log"}', file=sys.stderr)
    sys.exit(1)
finally:
    if proc.poll() is None:
        proc.terminate()
        try: proc.wait(timeout=15)
        except subprocess.TimeoutExpired: proc.kill()
    log.close()

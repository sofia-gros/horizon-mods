#!/usr/bin/env python3
"""Local smoke gate: launch the built, modded Horizon client and assert one thing.

Not run in CI (needs a built client and a prepared Horizon 26.3 resource folder).
Prints `SMOKE PASS` only when every step below really ran; otherwise it exits
non-zero and says which step failed. Nothing here claims more than it executed.

Steps: (1) pin check, (2) launch offscreen into a fresh test world, (3) wait for
the control endpoint and world readiness (= game started, mod hooks loaded),
(4) one assertion: the TConstruct `/tconstruct parts` command, injected by the
mod, turns 1 flint + 2 sticks into three TConstruct part items, (5) clean quit.
"""
import argparse, json, os, subprocess, sys, tempfile, time
from pathlib import Path

root = Path(__file__).resolve().parents[2]
p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True, help='horizon-client executable (modded build)')
p.add_argument('--ctl', type=Path, required=True, help='horizon-ctl executable')
p.add_argument('--resources', type=Path, required=True, help='prepared Horizon 26.3 resource folder')
p.add_argument('--run-dir', type=Path, default=root / '.local/smoke-runs')
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
       '--world', 'Probe', '--seed', '42', '--creative', '--test-terrain', '--render-distance', '2', '--width', '640', '--height', '360']
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

step = 'launch'
try:
    deadline = time.monotonic() + 120
    while not endpoint.exists():
        if proc.poll() is not None: raise RuntimeError(f'game exited early (code {proc.returncode})')
        if time.monotonic() > deadline: raise TimeoutError('control endpoint never appeared')
        time.sleep(.3)
    step = 'world ready'
    ctl('wait-ready', '120', timeout=125)
    step = 'assertion: /tconstruct parts'
    for c in ('clear @s', 'give @s minecraft:flint 1', 'give @s minecraft:stick 2'):
        ctl('command', c)
    ctl('chat', '/tconstruct parts'); time.sleep(1.5)
    counts = ctl('inventory')['counts']
    wanted = ('tconstruct:flint_pick_head', 'tconstruct:wood_binding', 'tconstruct:wood_handle')
    assert all(counts.get(x) == 1 for x in wanted), counts
    assert not counts.get('minecraft:flint') and not counts.get('minecraft:stick'), counts
    step = 'quit'
    ctl('quit'); proc.wait(timeout=60)
    assert proc.returncode == 0, f'exit code {proc.returncode}'
    print(json.dumps({'result': 'SMOKE PASS', 'run': str(run), 'parts': counts}, indent=2))
except BaseException as error:
    print(f'SMOKE FAIL at step "{step}": {error!r}\nlog: {run / "game.log"}', file=sys.stderr)
    sys.exit(1)
finally:
    if proc.poll() is None:
        proc.terminate()
        try: proc.wait(timeout=15)
        except subprocess.TimeoutExpired: proc.kill()
    log.close()

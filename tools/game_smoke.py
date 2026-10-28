#!/usr/bin/env python3
"""Exercise the prototype in a fresh single-player world."""
import argparse, json, os, subprocess, tempfile, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
p.add_argument('--split-sticks', action='store_true', help='put one stick in each of two slots')
p.add_argument('--output-parent', type=Path, default=Path('target'))
a = p.parse_args()
a.output_parent.mkdir(parents=True, exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='game-smoke-', dir=a.output_parent.resolve()))
endpoint = run / 'control.json'
env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off', RUST_MIN_STACK='16777216')
for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
    folder = run / key.lower(); folder.mkdir(); env[key] = str(folder)
command = [str(a.game.resolve()), '--resources', str(a.resources.resolve()), '--control-socket', str(endpoint), '--offscreen', '--world', 'Probe', '--seed', '42', '--creative', '--test-terrain', '--render-distance', '2', '--width', '640', '--height', '360']
if a.in_frame: command.append('--in-frame')
log = (run / 'game.log').open('w')
proc = subprocess.Popen(command, cwd=run, env=env, stdout=log, stderr=subprocess.STDOUT)
def ctl(*args, timeout=30):
    result = subprocess.run([str(a.ctl.resolve()), '--socket', str(endpoint), '--timeout', str(timeout), *args], capture_output=True, text=True, timeout=timeout+5, check=True)
    value = json.loads(result.stdout)
    if value.get('error'): raise RuntimeError(str(value))
    return value
try:
    deadline = time.monotonic() + 90
    while not endpoint.exists():
        if proc.poll() is not None: raise RuntimeError('game exited; see ' + str(run / 'game.log'))
        if time.monotonic() > deadline: raise TimeoutError('control endpoint')
        time.sleep(.2)
    ctl('wait-ready','90',timeout=95)
    commands = ('clear @s', 'item replace entity @s hotbar.0 with minecraft:flint 1', 'item replace entity @s hotbar.1 with minecraft:stick 1', 'item replace entity @s hotbar.2 with minecraft:stick 1') if a.split_sticks else ('clear @s','give @s minecraft:flint 1','give @s minecraft:stick 2')
    for cmd in commands:
        ctl('command',cmd)
    if a.split_sticks:
        slots = ctl('inventory')['hotbar']
        assert sum(1 for s in slots if s and s.get('item') == 'minecraft:stick' and s.get('count') == 1) == 2, slots
    ctl('chat','/tconstruct parts'); time.sleep(1)
    parts = ctl('inventory')['counts']
    assert all(parts.get(x)==1 for x in ('tconstruct:flint_pick_head','tconstruct:wood_binding','tconstruct:wood_handle')), parts
    assert not parts.get('minecraft:flint') and not parts.get('minecraft:stick'), parts
    ctl('chat','/tconstruct assemble'); time.sleep(1)
    assert ctl('inventory')['counts'].get('tconstruct:flint_pickaxe') == 1
    ctl('command','gamemode survival @s')
    ctl('command','setblock 0 71 2 minecraft:stone')
    ctl('command','tp @s 0.5 71 0.5'); time.sleep(1)
    mined=ctl('break','0','71','2','--timeout','30',timeout=35)
    held=ctl('inventory')['held']
    assert mined['block']=='minecraft:air' and held['damage']==1, (mined,held)
    ctl('save',timeout=30)
    ctl('open-inventory'); ctl('screenshot',str(run/'screen.png'))
    ctl('quit'); proc.wait(timeout=45)
    assert proc.returncode==0
    print(json.dumps({'ok': True, 'run': str(run), 'parts': parts, 'mining': mined, 'held': held, 'screenshot': str(run/'screen.png')}, indent=2))
finally:
    if proc.poll() is None:
        proc.terminate(); proc.wait(timeout=10)
    log.close()

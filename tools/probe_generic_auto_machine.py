"""Exercise tick-driven persistent machine storage in an isolated game."""
import argparse
import tempfile
import time
from pathlib import Path
from probe_horizon_server_mods import Game, ROOT

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--in-frame', action='store_true')
p.add_argument('--game', type=Path, default=ROOT.parent/'horizon/target/fast/horizon-client')
p.add_argument('--ctl', type=Path, default=ROOT.parent/'horizon/target/fast/horizon-ctl')
p.add_argument('--resources', type=Path, default=ROOT.parent/'horizon/.local/horizon-client/26.3')
p.add_argument('--client-mod', type=Path, default=ROOT/'examples/gui-storage-fixture/target/horizon-mod/gui_fixture.hmod')
p.add_argument('--server-mod', type=Path, default=ROOT/'examples/gui-storage-fixture/server/target/horizon-mod/gui_fixture_server.hmod')
args = p.parse_args()
(ROOT/'.local').mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='gui-auto-', dir=ROOT/'.local'))
game = None

def wait(label, condition):
    for _ in range(150):
        if condition():
            print('OK', label, flush=True)
            return
        time.sleep(.2)
    raise AssertionError(label)

def slots():
    return {s['slot']: s['stack'] for s in game.call('menu')['slots'] if s.get('stack')}

def click(slot, *flags):
    game.call('click-slot', str(slot), *flags)
    game.call('wait', '5')

def hotbar(item):
    return next(s['slot'] for s in game.call('inventory')['hotbar'] if s.get('item') == item)

try:
    game = Game(args, run, 'Miner', in_frame=args.in_frame)
    game.ready()
    for command in ['fill -2 70 -2 2 70 3 minecraft:stone', 'tp @s 0.5 71 0.5',
                    'setblock 0 71 2 gui_fixture:auto_machine', 'setblock 1 71 2 gui_fixture:foreign', 'clear @s',
                    'give @s minecraft:flint[custom_data={other_mod:{marker:7}}] 3',
                    'give @s minecraft:coal 1', 'gamemode survival @s']:
        game.call('command', command)
    game.call('interact', '0', '71', '2'); game.call('wait-for', 'screen', 'station', '--timeout', '15')
    click(hotbar('minecraft:flint')); click(600)
    game.call('wait', '60')
    wait('incomplete recipe changes nothing', lambda: slots().get(600, {}).get('count') == 3 and 602 not in slots())
    click(hotbar('minecraft:coal')); click(601)
    wait('automatic atomic conversion reflected in GUI', lambda: slots().get(600, {}).get('count') == 2 and 601 not in slots() and slots().get(602, {}).get('item') == 'minecraft:diamond')
    wait('duplicate, stale token and foreign access rejected', lambda: game.call('menu')['info']['mod_progress'] == [99, 99, 99])
    game.call('screenshot', str(run/'auto-machine.png'), timeout=30)
    game.call('command', 'give @s minecraft:diamond 63')
    wait('additional output available', lambda: game.call('inventory')['counts'].get('minecraft:diamond', 0) == 63)
    click(hotbar('minecraft:diamond')); click(602)
    wait('output buffer full', lambda: slots().get(602, {}).get('count') == 64)
    game.call('command', 'give @s minecraft:coal 1')
    wait('additional ingredient available', lambda: game.call('inventory')['counts'].get('minecraft:coal', 0) == 1)
    click(hotbar('minecraft:coal')); click(601)
    game.call('wait', '80')
    wait('full output leaves both inputs unchanged', lambda: slots().get(600, {}).get('count') == 2 and slots().get(601, {}).get('count') == 1 and slots().get(602, {}).get('count') == 64)
    click(602, 'shift')
    wait('processing resumes when output space opens', lambda: slots().get(600, {}).get('count') == 1 and 601 not in slots() and slots().get(602, {}).get('count') == 1)
    game.call('close'); game.call('save'); game.stop()
    game = Game(args, run, 'Miner', in_frame=args.in_frame, resume=True); game.ready()
    game.call('interact', '0', '71', '2'); game.call('wait-for', 'screen', 'station', '--timeout', '15')
    wait('storage survives restart', lambda: slots().get(600, {}).get('count') == 1 and slots().get(602, {}).get('item') == 'minecraft:diamond')
    click(600, 'shift')
    wait('unknown input data retained', lambda: any('other_mod:{marker:7}' in s.get('components', '') for s in game.call('inventory')['hotbar'] + game.call('inventory')['main']))
    game.call('close'); game.call('command', 'gamemode creative @s')
    game.call('break', '0', '71', '2', '--timeout', '30', timeout=35)
    game.call('command', 'tp @s 0.5 71 2.5')
    wait('removing block returns output', lambda: game.call('inventory')['counts'].get('minecraft:diamond', 0) == 65)
    game.call('command', 'setblock 0 71 2 gui_fixture:auto_machine')
    wait('machine replaced', lambda: game.call('block', '0', '71', '2')['block'] == 'gui_fixture:auto_machine')
    game.call('command', 'give @s minecraft:coal 1')
    game.call('command', 'tp @s 0.5 71 0.5')
    game.call('interact', '0', '71', '2'); game.call('wait-for', 'screen', 'station', '--timeout', '15')
    wait('replacement starts empty', lambda: not any(index in slots() for index in (600, 601, 602)))
    click(hotbar('minecraft:flint')); click(600)
    click(hotbar('minecraft:coal')); click(601)
    wait('replacement resumes processing', lambda: slots().get(602, {}).get('count') == 1 and 600 not in slots() and 601 not in slots())
    game.call('close')
    print('RUN', run, flush=True)
finally:
    if game: game.stop()

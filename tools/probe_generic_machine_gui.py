"""Exercise a generic multi-output machine GUI with atomic input consumption."""
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
run = Path(tempfile.mkdtemp(prefix='gui-machine-', dir=ROOT/'.local'))
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

try:
    game = Game(args, run, 'Miner', in_frame=args.in_frame)
    game.ready()
    for command in ['fill -2 70 -2 2 70 3 minecraft:stone', 'tp @s 0.5 71 0.5',
                    'setblock 0 71 2 gui_fixture:machine', 'clear @s',
                    'give @s minecraft:flint 2', 'give @s minecraft:coal 1', 'gamemode survival @s']:
        game.call('command', command)
    game.call('interact', '0', '71', '2')
    game.call('wait-for', 'screen', 'station', '--timeout', '15')
    first = game.call('menu')['info']['mod_progress'][0]
    wait('progress updates while open', lambda: game.call('menu')['info']['mod_progress'][0] != first)
    game.call('click-at', '245', '132')
    wait('button event updates progress', lambda: game.call('menu')['info']['mod_progress'] == [99])
    buffer_slot = next(s['slot'] for s in game.call('inventory')['hotbar'] if s.get('item') == 'minecraft:coal')
    click(buffer_slot); click(601)
    wait('mixed storage accepts item', lambda: slots().get(601, {}).get('item') == 'minecraft:coal')
    slot = next(s['slot'] for s in game.call('inventory')['hotbar'] if s.get('item') == 'minecraft:flint')
    click(slot); click(600)
    wait('both previews', lambda: slots().get(650, {}).get('item') == 'minecraft:diamond' and slots().get(651, {}).get('item') == 'minecraft:stick')
    game.call('screenshot', str(run/'machine.png'), timeout=30)
    game.call('command', 'give @s minecraft:stone 2304')
    wait('inventory full', lambda: game.call('inventory')['counts'].get('minecraft:stone', 0) == 2304)
    click(651, 'shift')
    wait('full inventory rejects entire transaction', lambda: slots().get(600, {}).get('count') == 2 and slots().get(650, {}).get('item') == 'minecraft:diamond' and slots().get(651, {}).get('item') == 'minecraft:stick')
    game.call('command', 'clear @s minecraft:stone')
    wait('inventory empty', lambda: game.call('inventory')['counts'].get('minecraft:stone', 0) == 0)
    click(651, 'shift')
    wait('both outputs received once', lambda: game.call('inventory')['counts'].get('minecraft:diamond', 0) == 2 and game.call('inventory')['counts'].get('minecraft:stick', 0) == 3 and slots().get(600, {}).get('count') == 1)
    game.call('close')
    wait('remaining input returned', lambda: game.call('inventory')['counts'].get('minecraft:flint', 0) == 1)
    game.call('save'); game.stop()
    game = Game(args, run, 'Miner', in_frame=args.in_frame, resume=True); game.ready()
    wait('all outputs survive reload', lambda: game.call('inventory')['counts'].get('minecraft:diamond', 0) == 2 and game.call('inventory')['counts'].get('minecraft:stick', 0) == 3 and game.call('inventory')['counts'].get('minecraft:flint', 0) == 1)
    game.call('interact', '0', '71', '2'); game.call('wait-for', 'screen', 'station', '--timeout', '15')
    wait('mixed storage survives reload', lambda: slots().get(601, {}).get('item') == 'minecraft:coal')
    game.call('click-at', '245', '132')
    wait('boost before replacement', lambda: game.call('menu')['info']['mod_progress'] == [99])
    game.call('close')
    game.call('command', 'setblock 0 71 2 minecraft:stone')
    wait('old block removed', lambda: game.call('block', '0', '71', '2')['block'] == 'minecraft:stone')
    game.call('command', 'setblock 0 71 2 gui_fixture:machine')
    wait('machine reinstalled', lambda: game.call('block', '0', '71', '2')['block'] == 'gui_fixture:machine')
    game.call('interact', '0', '71', '2'); game.call('wait-for', 'screen', 'station', '--timeout', '15')
    wait('replacement resets progress', lambda: game.call('menu')['info']['mod_progress'] == [50])
    game.call('close')
    print('RUN', run, flush=True)
finally:
    if game: game.stop()

"""Exercise a standalone seven-slot storage and progress GUI in an isolated world."""
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
run = Path(tempfile.mkdtemp(prefix='gui-storage-', dir=ROOT/'.local'))
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
                    'setblock 0 71 2 gui_fixture:storage', 'clear @s',
                    'give @s minecraft:diamond[custom_data={other_mod:{marker:7}}] 3', 'gamemode survival @s']:
        print(command, game.call('command', command), flush=True)
    print('block', game.call('block', '0', '71', '2'), flush=True)
    print('interact', game.call('interact', '0', '71', '2'), flush=True)
    game.call('wait-for', 'screen', 'station', '--timeout', '15')
    game.call('screenshot', str(run/'storage.png'), timeout=30)
    slot = next(s['slot'] for s in game.call('inventory')['hotbar'] if s.get('item') == 'minecraft:diamond')
    click(slot)
    click(606)
    wait('seven-slot storage accepts item', lambda: slots().get(606, {}).get('count') == 3)
    game.call('close')
    wait('closing does not return stored item', lambda: game.call('inventory')['counts'].get('minecraft:diamond', 0) == 0)
    game.call('save')
    game.stop()
    game = Game(args, run, 'Miner', in_frame=args.in_frame, resume=True)
    game.ready()
    game.call('interact', '0', '71', '2')
    game.call('wait-for', 'screen', 'station', '--timeout', '15')
    wait('storage persists after reload', lambda: slots().get(606, {}).get('count') == 3)
    click(606, 'shift')
    wait('item taken once', lambda: 606 not in slots() and game.call('inventory')['counts'].get('minecraft:diamond', 0) == 3)
    game.call('close')
    wait('foreign components preserved', lambda: any('other_mod:{marker:7}' in s.get('components', '') for s in game.call('inventory')['hotbar'] + game.call('inventory')['main']))
    game.call('interact', '0', '71', '2'); game.call('wait-for', 'screen', 'station', '--timeout', '15')
    slot = next(s['slot'] for s in game.call('inventory')['hotbar'] if s.get('item') == 'minecraft:diamond')
    click(slot); click(606)
    wait('stored before block break', lambda: slots().get(606, {}).get('count') == 3)
    game.call('close'); game.call('command', 'gamemode creative @s')
    game.call('break', '0', '71', '2', '--timeout', '30', timeout=35)
    game.call('command', 'tp @s 0.5 71 2.5')
    wait('block break returns stored items', lambda: game.call('inventory')['counts'].get('minecraft:diamond', 0) == 3)
    wait('spilled components preserved', lambda: any('other_mod:{marker:7}' in s.get('components', '') for s in game.call('inventory')['hotbar'] + game.call('inventory')['main']))
    print('RUN', run, flush=True)
finally:
    if game:
        game.stop()

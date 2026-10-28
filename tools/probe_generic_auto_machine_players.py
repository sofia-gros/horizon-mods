"""Race a GUI extraction with a tick-driven machine transaction."""
import argparse
import tempfile
import time
from pathlib import Path
from probe_horizon_server_mods import Game, ROOT

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--game', type=Path, default=ROOT.parent/'horizon/target/fast/horizon-client')
p.add_argument('--ctl', type=Path, default=ROOT.parent/'horizon/target/fast/horizon-ctl')
p.add_argument('--resources', type=Path, default=ROOT.parent/'horizon/.local/horizon-client/26.3')
p.add_argument('--client-mod', type=Path, default=ROOT/'examples/gui-storage-fixture/target/horizon-mod/gui_fixture.hmod')
p.add_argument('--server-mod', type=Path, default=ROOT/'examples/gui-storage-fixture/server/target/horizon-mod/gui_fixture_server.hmod')
args = p.parse_args()
(ROOT/'.local').mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='gui-auto-players-', dir=ROOT/'.local'))
games = []

def wait(label, condition):
    for _ in range(150):
        if condition():
            print('OK', label, flush=True)
            return
        time.sleep(.2)
    raise AssertionError(label)

def slots(game):
    return {s['slot']: s['stack'] for s in game.call('menu')['slots'] if s.get('stack') and 600 <= s['slot'] <= 602}

def click(game, slot, *flags):
    game.call('click-slot', str(slot), *flags)
    game.call('wait', '5')

def hotbar(game, item):
    return next(s['slot'] for s in game.call('inventory')['hotbar'] if s.get('item') == item)

try:
    host = Game(args, run, 'Host'); games.append(host); host.ready()
    lan = host.call('lan', 'open', '0', 'commands')
    port = lan.get('port') or host.call('lan', 'status').get('port')
    guest = Game(args, run, 'Guest', server=f'127.0.0.1:{port}'); games.append(guest); guest.ready()
    for command in ['gamemode creative @a', 'clear @a', 'fill -2 70 -2 2 70 3 minecraft:stone',
                    'tp Host -0.5 71 0.5', 'tp Guest 1.5 71 0.5',
                    'setblock 0 71 2 gui_fixture:auto_machine', 'give Host minecraft:flint 2',
                    'give Guest minecraft:coal 1', 'gamemode survival @a']:
        host.call('command', command)
    for game in games:
        wait(game.label+' machine visible', lambda: game.call('block', '0', '71', '2')['block'] == 'gui_fixture:auto_machine')
        game.call('interact', '0', '71', '2'); game.call('wait-for', 'screen', 'station', '--timeout', '15')
    click(host, hotbar(host, 'minecraft:flint')); click(host, 600)
    wait('guest sees input', lambda: slots(guest).get(600, {}).get('count') == 2)
    click(guest, hotbar(guest, 'minecraft:coal')); click(guest, 601)
    # The machine may commit before this extraction or the extraction may win.
    click(guest, 601, 'shift')
    wait('both views synchronize', lambda: slots(host) == slots(guest))
    host.call('wait', '40')
    wait('race settles', lambda: slots(host) == slots(guest))
    storage = slots(host)
    inventory = [game.call('inventory')['counts'] for game in games]
    flint = sum(i.get('minecraft:flint', 0) for i in inventory) + storage.get(600, {}).get('count', 0)
    coal = sum(i.get('minecraft:coal', 0) for i in inventory) + storage.get(601, {}).get('count', 0)
    diamond = sum(i.get('minecraft:diamond', 0) for i in inventory) + storage.get(602, {}).get('count', 0)
    assert diamond in (0, 1) and flint == 2 - diamond and coal == 1 - diamond, (flint, coal, diamond)
    print('OK GUI race conserves inputs and output:', flint, coal, diamond, flush=True)
    for game in games: game.call('close')
    print('RUN', run, flush=True)
finally:
    for game in reversed(games): game.stop()

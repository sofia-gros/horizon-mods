"""Exercise shared generic storage with two simultaneous LAN players."""
import argparse
from concurrent.futures import ThreadPoolExecutor
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
run = Path(tempfile.mkdtemp(prefix='gui-storage-players-', dir=ROOT/'.local'))
games = []
def wait(label, condition):
    for _ in range(150):
        if condition():
            print('OK', label, flush=True)
            return
        time.sleep(.2)
    raise AssertionError(label)
def slots(game):
    return {s['slot']: s['stack'] for s in game.call('menu')['slots'] if s.get('stack')}
def click(game, slot, *flags):
    game.call('click-slot', str(slot), *flags)
    game.call('wait', '5')
try:
    host = Game(args, run, 'Host'); games.append(host); host.ready()
    lan = host.call('lan', 'open', '0', 'commands')
    port = lan.get('port') or host.call('lan', 'status').get('port')
    guest = Game(args, run, 'Guest', server=f'127.0.0.1:{port}'); games.append(guest); guest.ready()
    for command in ['gamemode creative @a', 'clear @a', 'fill -2 70 -2 2 70 3 minecraft:stone',
                    'tp Host -0.5 71 0.5', 'tp Guest 1.5 71 0.5', 'setblock 0 71 2 gui_fixture:storage',
                    'give Host minecraft:diamond 3', 'gamemode survival @a']:
        host.call('command', command)
    for game in games:
        wait(game.label+' block visible', lambda: game.call('block','0','71','2').get('block') == 'gui_fixture:storage')
        game.call('interact', '0', '71', '2')
        game.call('wait-for', 'screen', 'station', '--timeout', '15')
    slot = next(s['slot'] for s in host.call('inventory')['hotbar'] if s.get('item') == 'minecraft:diamond')
    click(host, slot); click(host, 606)
    wait('guest observes host insertion', lambda: slots(guest).get(606, {}).get('count') == 3)
    click(guest, 606, 'shift')
    wait('host observes guest removal', lambda: 606 not in slots(host))
    wait('guest receives exactly three', lambda: guest.call('inventory')['counts'].get('minecraft:diamond', 0) == 3)
    host.call('command', 'give Host minecraft:diamond 3')
    wait('host has another stack', lambda: host.call('inventory')['counts'].get('minecraft:diamond', 0) == 3)
    slot = next(s['slot'] for s in host.call('inventory')['hotbar'] if s.get('item') == 'minecraft:diamond')
    click(host, slot); click(host, 606)
    wait('both see contested stack', lambda: slots(guest).get(606, {}).get('count') == 3 and slots(host).get(606, {}).get('count') == 3)
    with ThreadPoolExecutor(max_workers=2) as pool:
        futures = [pool.submit(click, game, 606, 'shift') for game in games]
        for future in futures: future.result()
    wait('contested stack taken', lambda: 606 not in slots(host) and 606 not in slots(guest))
    wait('no double consumption', lambda: sum(game.call('inventory')['counts'].get('minecraft:diamond', 0) for game in games) == 6)
    for game in games: game.call('close')
    assert sum(game.call('inventory')['counts'].get('minecraft:diamond', 0) for game in games) == 6
    print('OK no duplication on close', flush=True)
    print('RUN', run, flush=True)
finally:
    for game in reversed(games): game.stop()

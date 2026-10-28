"""Exercise shared generic storage with two simultaneous LAN players."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import tempfile
import time
from pathlib import Path
from probe_horizon_server_mods import Game, ROOT

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--in-frame', action='store_true')
p.add_argument('--game', type=Path, default=ROOT.parent/'horizon/target/fast/horizon-client')
p.add_argument('--ctl', type=Path, default=ROOT.parent/'horizon/target/fast/horizon-ctl')
p.add_argument('--resources', type=Path, default=ROOT.parent/'horizon/.local/horizon-client/26.3')
p.add_argument('--client-mod', type=Path, default=ROOT/"Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
p.add_argument('--server-mod', type=Path, default=ROOT/"Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod")
args = p.parse_args()
args.width = 1280
args.height = 720
(ROOT/'.local').mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='tconstruct-station-players-', dir=ROOT/'.local'))
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
    host = Game(args, run, 'Host', in_frame=args.in_frame); games.append(host); host.ready()
    lan = host.call('lan', 'open', '0', 'commands')
    port = lan.get('port') or host.call('lan', 'status').get('port')
    guest = Game(args, run, 'Guest', server=f'127.0.0.1:{port}'); games.append(guest); guest.ready()
    for command in ['gamemode creative @a', 'clear @a', 'fill -2 70 -2 2 70 3 minecraft:stone',
                    'tp @s -0.5 71 0.5', 'tp Guest 1.5 71 0.5', 'setblock 0 71 2 tconstruct:tinker_station']:
        host.call('command', command)
    for item, kind, material in [('pick_head', 'Head', 'flint'), ('tool_handle', 'Handle', 'wood'), ('tool_binding', 'Binding', 'wood')]:
        payload = f'{{"kind":"{kind}","material":"tconstruct:{material}"}}'
        host.call('command', f"give @s tconstruct:{item}[custom_data={{tconstruct:{{state_json:'{payload}'}}}}] 1")
    host.call('command', 'gamemode survival @a')
    for game in games:
        wait(game.label+' block visible', lambda: game.call('block','0','71','2').get('block') == 'tconstruct:tinker_station')
        game.call('interact', '0', '71', '2')
        game.call('wait-for', 'screen', 'station', '--timeout', '15')
    host.call('click-gui', '109', '73'); host.call('wait', '10')
    for index, item in enumerate(['tconstruct:pick_head','tconstruct:tool_handle','tconstruct:tool_binding']):
        slot = next(s['slot'] for s in host.call('inventory')['hotbar'] if s.get('item') == item)
        click(host, slot); click(host, 601 + index)
    wait('both see assembled preview', lambda: all(slots(game).get(650,{}).get('item') == 'minecraft:wooden_pickaxe' for game in games))
    for game, name in [(host, 'HostBlade'), (guest, 'GuestBlade')]:
        game.call('click-gui','90','10'); game.call('type', name); game.call('wait','10')
    wait('names stay player-local', lambda: 'HostBlade' in str(slots(host).get(650,{})) and 'GuestBlade' in str(slots(guest).get(650,{})))
    guest.call('click-gui','90','73')
    wait('guest selection clears both previews', lambda: all(650 not in slots(game) for game in games))
    host.call('click-gui','109','73')
    wait('switching back does not restore stale names', lambda: all(slots(game).get(650,{}).get('item') == 'minecraft:wooden_pickaxe' and slots(game)[650].get('custom_name') is None for game in games))
    for game, name in [(host, 'HostBlade'), (guest, 'GuestBlade')]:
        game.call('click-gui','90','10'); game.call('type', name); game.call('wait','10')
    wait('new names stay player-local', lambda: 'HostBlade' in str(slots(host).get(650,{})) and 'GuestBlade' in str(slots(guest).get(650,{})))
    with ThreadPoolExecutor(max_workers=2) as pool:
        futures = [pool.submit(click, game, 650, 'shift') for game in games]
        for future in futures: future.result()
    wait('exactly one crafted tool', lambda: sum(game.call('inventory')['counts'].get('minecraft:wooden_pickaxe', 0) for game in games) == 1)
    wait('shared ingredients consumed once', lambda: all(601 not in slots(game) and 602 not in slots(game) and 603 not in slots(game) for game in games))
    print('OK independent names and atomic competing result', flush=True)
    print('RUN', run, flush=True)
finally:
    for game in reversed(games): game.stop()

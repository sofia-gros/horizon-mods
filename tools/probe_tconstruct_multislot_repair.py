"""Exercise ordered three-slot repair, invalid input, persistence and LAN contention."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import tempfile
import time
from probe_horizon_server_mods import Game, ROOT
from probe_horizon_item_state import states

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--in-frame', action='store_true')
p.add_argument('--game', type=Path, default=ROOT.parent/'horizon/target/fast/horizon-client')
p.add_argument('--ctl', type=Path, default=ROOT.parent/'horizon/target/fast/horizon-ctl')
p.add_argument('--resources', type=Path, default=ROOT.parent/'horizon/.local/horizon-client/26.3')
p.add_argument('--client-mod', type=Path, default=ROOT/"Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
p.add_argument('--server-mod', type=Path, default=ROOT/"Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod")
args = p.parse_args(); args.width = 1280; args.height = 720
(ROOT/'.local').mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='multislot-repair-', dir=ROOT/'.local'))
games = []
def wait(label, condition):
    for _ in range(150):
        if condition(): print('OK', label, flush=True); return
        time.sleep(.2)
    raise AssertionError(label)
def slots(game):
    return {s['slot']: s['stack'] for s in game.call('menu')['slots'] if s.get('stack')}
def click(game, slot, *flags):
    game.call('click-slot', str(slot), *flags); game.call('wait', '5')
def hotbar(game, item):
    return next(s['slot'] for s in game.call('inventory')['hotbar'] if s.get('item') == item)
def open_station(game):
    game.call('interact', '0', '71', '2'); game.call('wait-for', 'screen', 'station', '--timeout', '15')
def select_repair(game):
    game.call('click-gui', '90', '73'); game.call('wait', '5')
try:
    host = Game(args, run, 'Miner', in_frame=args.in_frame); games.append(host); host.ready()
    for command in ['fill -2 70 -2 2 70 3 minecraft:stone', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:tinker_station', 'gamemode creative @s', 'clear @s']:
        host.call('command', command)
    host.call('chat', '/tconstruct specimen flint wood wood 85')
    wait('damaged specimen', lambda: [s['damage'] for s in states(host.call('inventory'))] == [85])
    host.call('select','1'); host.call('chat','/tconstruct_tool activate')
    wait('tool profile active', lambda: host.call('inventory')['held'].get('damage') == 85)
    for command in ['item replace entity @s hotbar.1 with minecraft:flint 1', 'item replace entity @s hotbar.2 with minecraft:flint 2', 'item replace entity @s hotbar.3 with minecraft:stone 1', 'gamemode survival @s']:
        host.call('command', command)
    wait('three material stacks available', lambda: host.call('inventory')['counts'].get('minecraft:flint') == 3 and host.call('inventory')['counts'].get('minecraft:stone') == 1)
    open_station(host); select_repair(host)
    click(host,hotbar(host,'minecraft:wooden_pickaxe')); click(host,600)
    click(host,hotbar(host,'minecraft:flint')); click(host,601)
    click(host,hotbar(host,'minecraft:flint')); click(host,603)
    wait('split stacks preview fully repaired', lambda: slots(host).get(650,{}).get('damage') == 0 and slots(host).get(601,{}).get('count') == 1 and slots(host).get(603,{}).get('count') == 2)
    host.call('wait', '100')
    host.call('screenshot', str(run/'multislot-preview.png'), timeout=30)
    click(host,hotbar(host,'minecraft:stone')); click(host,602)
    wait('invalid middle input rejects preview', lambda: 650 not in slots(host) and slots(host).get(602,{}).get('item') == 'minecraft:stone')
    click(host,602,'shift')
    wait('removing invalid input restores preview', lambda: slots(host).get(650,{}).get('damage') == 0)
    host.call('close'); host.call('save'); host.stop(); games.remove(host)
    host = Game(args, run, 'Miner', in_frame=args.in_frame, resume=True); games.append(host); host.ready()
    open_station(host); select_repair(host)
    wait('split inputs and preview survive reload', lambda: slots(host).get(650,{}).get('damage') == 0 and slots(host).get(601,{}).get('count') == 1 and slots(host).get(603,{}).get('count') == 2)
    lan = host.call('lan','open','0','commands'); port = lan.get('port') or host.call('lan','status').get('port')
    guest = Game(args,run,'Guest',server=f'127.0.0.1:{port}'); games.append(guest); guest.ready()
    host.call('command','tp Guest 1.5 71 0.5')
    wait('guest sees station',lambda:guest.call('block','0','71','2').get('block') == 'tconstruct:tinker_station')
    open_station(guest)
    wait('both see same repair preview', lambda: all(slots(g).get(650,{}).get('damage') == 0 for g in games))
    with ThreadPoolExecutor(max_workers=2) as pool:
        futures = [pool.submit(click, g, 650, 'shift') for g in games]
        for future in futures: future.result()
    wait('one repaired tool, ordered consumption and no duplication', lambda: sum(g.call('inventory')['counts'].get('minecraft:wooden_pickaxe',0) for g in games) == 1 and all(600 not in slots(g) and 601 not in slots(g) and slots(g).get(603,{}).get('count') == 1 for g in games))
    assert any(s['damage'] == 0 for g in games for s in states(g.call('inventory')))
    print('PASS multi-slot repair', run, flush=True)
finally:
    for game in reversed(games): game.stop()

"""Check adjacent side slots and transactions across two players."""
import argparse
import tempfile
import time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='builder-side-lan-', dir=Path('target').resolve()))
print('run', run, flush=True)
games = []
def slots(g): return {row['slot']: row['stack'] for row in g.call('menu')['slots'] if row.get('stack')}
def click(g, slot, *flags): g.call('click-slot', str(slot), *flags); g.call('wait', '3')
def wait(label, predicate):
    for _ in range(120):
        if predicate(): return
        time.sleep(.2)
    raise AssertionError(label)
try:
    host = Game(a, run, 'Host', in_frame=a.in_frame); games.append(host); host.ready()
    port = host.call('lan', 'open', '0', 'commands')['port']
    guest = Game(a, run, 'Guest', server='127.0.0.1:'+str(port)); games.append(guest); guest.ready()
    for command in ('fill -2 70 -2 3 70 3 minecraft:stone', 'clear @a',
                    'tp @s -0.5 71 0.5', 'tp Guest 1.5 71 0.5',
                    'setblock 0 71 2 tconstruct:part_builder', 'setblock 1 71 2 minecraft:chest',
                    'give @s tconstruct:pattern 1', 'give @s minecraft:oak_log 1', 'give Guest minecraft:stick 1'):
        host.call('command', command)
    for g in games:
        wait(g.label+' builder', lambda g=g: g.call('block','0','71','2').get('block') == 'tconstruct:part_builder')
        g.call('interact', '0', '71', '2'); g.call('wait', '10')
    guest_stick = next(s['slot'] for s in guest.call('inventory')['hotbar'] if s.get('item') == 'minecraft:stick')
    click(guest, guest_stick); click(guest, 327)
    wait('side synchronization', lambda: all(slots(g).get(327, {}).get('item') == 'minecraft:stick' for g in games))
    guest.call('screenshot', str(run / 'guest-side.png'))
    click(host, 0); click(host, 300); click(host, 1); click(host, 301)
    wait('shared inputs', lambda: all(slots(g).get(300, {}).get('item') == 'tconstruct:pattern' and slots(g).get(301, {}).get('item') == 'minecraft:oak_log' for g in games))
    wait('choice', lambda: any(n.get('label') == 'Tool Handle' and n.get('enabled') for n in guest.call('ui')['widgets']))
    guest.call('click', next(n['id'] for n in guest.call('ui')['widgets'] if n.get('label') == 'Tool Handle' and n.get('enabled')))
    host.call('wait', '15'); guest.call('wait', '15')
    def take(g): click(g, 322, 'shift')
    with ThreadPoolExecutor(max_workers=2) as pool:
        for f in [pool.submit(take, g) for g in games]: f.result()
    total = sum(sum(s.get('count', 0) for s in g.call('inventory')['hotbar'] if s.get('item') == 'tconstruct:tool_handle') for g in games)
    total += sum(row.get('stack', {}).get('count', 0) for row in host.call('menu')['slots'] if 327 <= row['slot'] < 354 and row.get('stack', {}).get('item') == 'tconstruct:tool_handle')
    assert total == 1, total
    for g in reversed(games): g.stop()
    print('PASS side synchronization and concurrent result', flush=True)
finally:
    for g in reversed(games): g.cleanup()

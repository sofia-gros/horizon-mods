import argparse, json, sys, time, re
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
for k in ('game', 'ctl', 'resources'):
    p.add_argument('--' + k, type=Path, required=True)
p.add_argument('--run-dir', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
if not a.run_dir.resolve().is_relative_to((Path(__file__).resolve().parents[2] / 'target').resolve()):
 raise SystemExit('run-dir must be inside tconstruct/target')
run = a.run_dir.resolve()
games = []

def cmd(g, x):
    r = g.call('command', x)
    g.call('wait', '8')
    return r

def click(g, s):
    g.call('click-slot', str(s))
    g.call('wait', '8')

def hotbar(g, item):
    return next((s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item') == item))

def slots(g):
    return {s['slot']: s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}

def wait(predicate):
    for _ in range(100):
        if predicate():
            return
        time.sleep(0.2)
    print('GUEST STATUS', json.dumps(q.call('status')), flush=True)
    print('GUEST MENU', json.dumps(q.call('menu')), flush=True)
    q.call('screenshot',str(run/'reinforced-lan-diagnostic.png'))
    print('HOST BLOCK', json.dumps(h.call('block', '0', '71', '2')), flush=True)
    raise AssertionError('timed out waiting for station state')
try:
    h = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)
    games.append(h)
    h.ready()
    port = h.call('lan', 'open', '0', 'commands')['port']
    q = Game(a, run, 'Guest', server='127.0.0.1:' + str(port), resume=(run / 'Guest').exists())
    games.append(q)
    q.ready()
    for x in ('gamemode creative @a', 'gamerule minecraft:spawn_mobs false', 'kill @e[type=minecraft:slime]', 'kill @e[type=minecraft:zombie]', 'clear @a', 'fill -2 71 -1 2 73 3 minecraft:air', 'tp @s -0.5 71 0.5', 'tp Guest 1.5 71 0.5', 'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:scorched_anvil', 'give Guest tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85,tic_attack_damage:1.75f,tic_attack_speed:1.2f},minecraft:max_damage=85] 1', 'give Guest tconstruct:emerald_reinforcement 1'):
        cmd(h, x)
    wait(lambda: q.call('block', '0', '71', '2').get('block') == 'tconstruct:scorched_anvil')
    q.call('hotbar','9');q.call('wait','10')
    print('interact', q.call('interact', '0', '71', '2'), flush=True)
    wait(lambda: any((w['id'] == 'slot-300' for w in q.call('ui')['widgets'])))
    print('station open', flush=True)
    q.call('wait', '15')
    click(q, hotbar(q, 'tconstruct:emerald_reinforcement'))
    click(q, 305)
    wait(lambda: slots(q).get(305, {}).get('item') == 'tconstruct:emerald_reinforcement')
    click(q, hotbar(q, 'tconstruct:pickaxe'))
    click(q, 300)
    wait(lambda: slots(q).get(325, {}).get('item') == 'tconstruct:pickaxe')
    click(q, 325)
    assert (q.call('menu')['carried'] or {}).get('item') == 'tconstruct:pickaxe'
    click(q, 0)
    q.call('close')
    q.call('hotbar', '1')
    q.call('wait', '10')
    held = q.call('inventory')['held']
    print('guest partial', json.dumps(held), flush=True)
    assert 'tic_upgrades' in held.get('components', '') and re.search('reinforced[^0-9]*1', held.get('components', ''))
    cmd(h, 'give Guest tconstruct:emerald_reinforcement 4')
    q.call('wait', '10')
    q.call('interact', '0', '71', '2')
    q.call('wait', '10')
    click(q, 0)
    click(q, 300)
    click(q, hotbar(q, 'tconstruct:emerald_reinforcement'))
    click(q, 301)
    wait(lambda: slots(q).get(325, {}).get('item') == 'tconstruct:pickaxe')
    click(q, 325)
    assert (q.call('menu')['carried'] or {}).get('item') == 'tconstruct:pickaxe'
    click(q, 0)
    assert slots(q).get(301, {}).get('item') == 'tconstruct:emerald_reinforcement', slots(q)
    assert slots(q)[301].get('count') == 1, slots(q)
    click(q, 301)
    click(q, 1)
    q.call('screenshot', str(run / 'reinforced-lan.png'))
    q.call('close')
    q.call('wait', '10')
    held = q.call('inventory')['held']
    print('guest full', json.dumps(held), 'counts', json.dumps(q.call('inventory')['counts']), flush=True)
    assert re.search('reinforced[^0-9]*4', held.get('components', ''))
    assert q.call('inventory')['counts'].get('tconstruct:emerald_reinforcement', 0) == 1
    for w in reversed(games):
        w.stop()
    print('PASS LAN Reinforced partial and remainder', flush=True)
finally:
    for w in reversed(games):
        w.cleanup()

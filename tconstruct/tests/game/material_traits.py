import argparse, json, time, re
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
for k in ('game', 'ctl', 'resources'):
    p.add_argument('--' + k, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)

def cmd(x):
    r = g.call('command', x)
    g.call('wait', '6')
    return r

def click(slot):
    g.call('click-slot', str(slot))
    g.call('wait', '5')

def hotbar(item):
    return next((s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item') == item))

def held_tool(materials, damage):
    mat = ','.join(('"tconstruct:' + x + '"' for x in materials))
    return 'tconstruct:pickaxe[minecraft:damage=' + str(damage) + ',minecraft:max_damage=200,minecraft:custom_data={tic_materials:[' + mat + '],tic_attack_damage:1.75f,tic_attack_speed:1.2f,tic_max_damage:200}]'

def prep():
    for x in ('gamemode creative @s', 'effect clear @s', 'effect give @s minecraft:instant_health 1 4 true', 'kill @e[type=minecraft:zombie]', 'clear @s', 'tp @s 0.5 71 0.5'):
        cmd(x)
    g.call('hotbar', '1')
    g.call('wait', '3')

def attack(materials, damage):
    prep()
    cmd('item replace entity @s weapon.mainhand with ' + held_tool(materials, damage))
    print('held', json.dumps(g.call('inventory')['held']), flush=True)
    cmd('gamemode survival @s')
    cmd('summon minecraft:zombie 1.5 71 0.5 {NoAI:1b}')
    g.call('wait', '30')
    g.call('hurtlog')
    entities = g.call('entities', '8', 'minecraft:zombie')['entities']
    g.call('attack', str(entities[0]['id']))
    log = g.call('hurtlog')
    entry = log['players'][0]['attacks'][-1]
    print('attack', materials, damage, json.dumps(entry), flush=True)
    return entry['amount']

def repair(materials):
    prep()
    cmd('setblock 0 71 2 minecraft:air')
    cmd('setblock 0 71 2 tconstruct:tinker_station')
    cmd('item replace entity @s weapon.mainhand with ' + held_tool(materials, 60))
    cmd('give @s minecraft:flint 1')
    g.call('interact', '0', '71', '2')
    g.call('wait', '10')
    click(0)
    click(300)
    click(hotbar('minecraft:flint'))
    click(301)
    click(325)
    out = g.call('menu')['carried'] or {}
    print('repair', materials, json.dumps(out), flush=True)
    assert out.get('item') == 'tconstruct:pickaxe'
    g.call('screenshot', str(run / 'material-traits-repair.png'))
    g.call('close')
    return out.get('damage', 0)
try:
    g.ready()
    if g.call('status')['screen'] == 'dead':
        g.call('respawn')
        g.call('wait', '20')
    fresh = attack(['flint', 'wood', 'wood'], 0)
    jagged = attack(['flint', 'wood', 'wood'], 64)
    stone = attack(['rock', 'wood', 'wood'], 64)
    print('amounts', fresh, jagged, stone, flush=True)
    assert jagged > fresh > stone
    base = repair(['flint', 'flint', 'flint'])
    cultivated = repair(['flint', 'wood', 'wood'])
    preserved = repair(['flint', 'treated_wood', 'flint'])
    print('repair results', base, cultivated, preserved, flush=True)
    assert cultivated < preserved < base
    g.stop()
    print('PASS trait attack and repair', flush=True)
finally:
    g.cleanup()

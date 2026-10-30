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

def hit_current():
    cmd('gamemode creative @s')
    cmd('kill @e[type=minecraft:zombie]')
    cmd('tp @s 0.5 71 0.5')
    cmd('gamemode survival @s')
    cmd('summon minecraft:zombie 1.5 71 0.5 {NoAI:1b}')
    g.call('wait', '30')
    g.call('hurtlog')
    entities = g.call('entities', '8', 'minecraft:zombie')['entities']
    g.call('attack', str(entities[0]['id']))
    entry = g.call('hurtlog')['players'][0]['attacks'][-1]
    return entry['amount']
try:
    g.ready()
    if g.call('status')['screen'] == 'dead':
        g.call('respawn')
        g.call('wait', '20')
    prep()
    cmd('setblock 0 71 2 minecraft:air')
    cmd('setblock 0 71 2 tconstruct:tinker_station')
    cmd('item replace entity @s weapon.mainhand with ' + held_tool(['flint', 'wood', 'wood'], 64))
    cmd('give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:wood"}] 1')
    before = hit_current()
    print('before swap attack', before, flush=True)
    cmd('gamemode creative @s')
    g.call('interact', '0', '71', '2')
    g.call('wait', '10')
    click(0)
    click(300)
    click(hotbar('tconstruct:pick_head'))
    click(301)
    click(325)
    out = g.call('menu')['carried'] or {}
    print('swap output', json.dumps(out), flush=True)
    assert out.get('item') == 'tconstruct:pickaxe'
    click(0)
    g.call('close')
    after = hit_current()
    print('after swap attack', after, flush=True)
    assert abs(after - 1.5) < 0.001
    g.call('screenshot', str(run / 'material-traits-swap.png'))
    g.call('save')
    g.stop()
    g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)
    g.ready()
    saved = g.call('inventory')['held']
    print('saved', json.dumps(saved), flush=True)
    assert 'tconstruct:wood' in saved.get('components', '')
    after_reload = hit_current()
    print('after reload attack', after_reload, flush=True)
    assert abs(after_reload - 1.5) < 0.001
    g.stop()
    print('PASS trait swap and reload', flush=True)
finally:
    g.cleanup()

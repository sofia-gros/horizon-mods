import argparse, json, re, sys, time
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
games = [g]

def cmd(x):
    r = g.call('command', x)
    g.call('wait', '6')
    return r

def click(w, s):
    w.call('click-slot', str(s))
    w.call('wait', '5')

def hotbar(w, item):
    return next((s['slot'] for s in w.call('inventory')['hotbar'] if s.get('item') == item))

def show(w):
    return {s['slot']: s['stack'] for s in w.call('menu')['slots'] if s.get('stack')}

def value(item, name):
    m = re.search('\\b' + name + ':(-?[0-9.]+)f?', item.get('components', ''))
    assert m, (name, item)
    return float(m.group(1))

def upgrade(item, ingredient):
    cmd('give @s ' + ingredient)
    g.call('interact', '0', '71', '2')
    g.call('wait', '10')
    click(g, 0)
    click(g, 300)
    click(g, hotbar(g, item))
    click(g, 301)
    print('preview', ingredient, json.dumps(show(g)), flush=True)
    click(g, 325)
    out = g.call('menu')['carried'] or {}
    assert out.get('item') == 'tconstruct:pickaxe', out
    click(g, 0)
    g.call('screenshot', str(run / 'station-upgrades.png'))
    g.call('close')
    return g.call('inventory')['held']

def attackpair():
    cmd('gamemode creative @s')
    cmd('item modify entity @s weapon.mainhand {type:"minecraft:set_damage",function:"minecraft:set_damage",damage:1.0}')
    cmd('kill @e[type=minecraft:zombie]')
    cmd('tp @s 0.5 71 0.5')
    cmd('gamemode survival @s')
    cmd('summon minecraft:zombie 1.5 71 0.5 {NoAI:1b}')
    g.call('wait', '35')
    g.call('hurtlog')
    id = g.call('entities', '8', 'minecraft:zombie')['entities'][0]['id']
    g.call('attack', str(id))
    g.call('wait', '5')
    g.call('attack', str(id))
    return g.call('hurtlog')['players'][0]['attacks']
try:
    g.ready()
    if g.call('status')['screen'] == 'dead':
        g.call('respawn')
        g.call('wait', '20')
    for x in ('gamemode creative @s', 'effect clear @s', 'effect give @s minecraft:instant_health 1 4 true', 'kill @e[type=minecraft:zombie]', 'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:tinker_station', 'clear @s', 'kill @e[type=minecraft:item]', 'tp @s 0.5 71 0.5', 'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1', 'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1', 'give @s tconstruct:tool_binding[minecraft:custom_data={Material:"tconstruct:wood"}] 1'):
        cmd(x)
    g.call('hotbar', '1')
    g.call('interact', '0', '71', '2')
    g.call('wait', '10')
    for i, item in enumerate(('tconstruct:pick_head', 'tconstruct:tool_handle', 'tconstruct:tool_binding')):
        click(g, hotbar(g, item))
        click(g, 300 + i)
    click(g, 324)
    assert (g.call('menu')['carried'] or {}).get('item') == 'tconstruct:pickaxe'
    click(g, 0)
    g.call('close')
    g.call('hotbar', '1')
    baseline = g.call('inventory')['held']
    print('baseline', json.dumps(baseline), flush=True)
    assert abs(value(baseline, 'tic_attack_damage') - 1.75) < 0.001
    base_hits = attackpair()
    print('base hits', json.dumps(base_hits), flush=True)
    cmd('gamemode creative @s')
    partial = upgrade('minecraft:quartz', 'minecraft:quartz 35')
    print('sharp partial', json.dumps(partial), flush=True)
    assert abs(value(partial, 'tic_attack_damage') - (1.75 + 0.75 * 35 / 36)) < 0.001
    complete = upgrade('minecraft:quartz_block', 'minecraft:quartz_block 1')
    print('sharp complete', json.dumps(complete), flush=True)
    assert abs(value(complete, 'tic_attack_damage') - 2.5) < 0.001
    g.call('wait', '30')
    assert g.call('inventory')['counts'].get('minecraft:quartz', 0) == 3
    swift_partial = upgrade('minecraft:amethyst_shard', 'minecraft:amethyst_shard 36')
    print('swift partial', json.dumps(swift_partial), flush=True)
    assert abs(value(swift_partial, 'tic_attack_speed') - 1.23) < 0.001
    swift_full = upgrade('minecraft:amethyst_shard', 'minecraft:amethyst_shard 36')
    print('swift full', json.dumps(swift_full), flush=True)
    assert abs(value(swift_full, 'tic_attack_speed') - 1.26) < 0.001
    sharp2 = upgrade('minecraft:quartz', 'minecraft:quartz 33')
    print('sharp second', json.dumps(sharp2), flush=True)
    assert abs(value(sharp2, 'tic_attack_damage') - 3.25) < 0.001
    hits = attackpair()
    print('upgraded hits', json.dumps(hits), flush=True)
    assert abs(hits[0]['amount'] - 4.25) < 0.001
    assert hits[-1]['scale'] > base_hits[-1]['scale']
    cmd('gamemode creative @s')
    for invalid in ('minecraft:amethyst_shard', 'minecraft:dirt'):
        cmd('give @s ' + invalid + ' 1')
        source = hotbar(g, invalid)
        g.call('interact', '0', '71', '2')
        g.call('wait', '10')
        click(g, 0)
        click(g, 300)
        click(g, source)
        click(g, 301)
        assert 325 not in show(g), show(g)
        click(g, 301)
        click(g, source)
        click(g, 300)
        click(g, 0)
        g.call('close')
        print('rejected', invalid, flush=True)
    cmd('give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:wood"}] 1')
    g.call('interact', '0', '71', '2')
    g.call('wait', '10')
    click(g, 0)
    click(g, 300)
    click(g, hotbar(g, 'tconstruct:pick_head'))
    click(g, 301)
    click(g, 325)
    assert (g.call('menu')['carried'] or {}).get('item') == 'tconstruct:pickaxe'
    click(g, 0)
    g.call('close')
    swapped = g.call('inventory')['held']
    print('swap', json.dumps(swapped), flush=True)
    assert 'tic_upgrades' in swapped.get('components', '') and abs(value(swapped, 'tic_attack_damage') - 2.0) < 0.001 and (abs(value(swapped, 'tic_attack_speed') - 1.26) < 0.001)
    g.call('save')
    g.stop()
    g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)
    games = [g]
    g.ready()
    saved = g.call('inventory')['held']
    print('saved', json.dumps(saved), flush=True)
    assert 'tic_upgrades' in saved.get('components', '') and abs(value(saved, 'tic_attack_damage') - 2.0) < 0.001
    print('PASS singleplayer', flush=True)
    g.stop()
finally:
    for w in reversed(games):
        w.cleanup()

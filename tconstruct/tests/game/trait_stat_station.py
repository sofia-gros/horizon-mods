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
pack = g.root / 'saves/Probe/datapacks/trait-stat-test'
created = False

def cmd(x):
    r = g.call('command', x)
    g.call('wait', '6')
    return r

def click(slot):
    g.call('click-slot', str(slot))
    g.call('wait', '5')

def hotbar(item):
    return next((s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item') == item))

def set_trait(name):
    f = pack / 'data/tconstruct/tinkering/materials/traits/flint.json'
    f.write_text(json.dumps({'default': [{'name': 'tconstruct:' + name, 'level': 1}]}))
    cmd('reload')
    g.call('wait', '50')

def build():
    for x in ('gamemode creative @s', 'effect clear @s', 'effect give @s minecraft:instant_health 1 4 true', 'kill @e[type=minecraft:zombie]', 'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:tinker_station', 'clear @s', 'kill @e[type=minecraft:item]', 'tp @s 0.5 71 0.5', 'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1', 'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:flint"}] 1', 'give @s tconstruct:tool_binding[minecraft:custom_data={Material:"tconstruct:flint"}] 1'):
        cmd(x)
    g.call('hotbar', '1')
    g.call('interact', '0', '71', '2')
    g.call('wait', '10')
    for idx, item in enumerate(('tconstruct:pick_head', 'tconstruct:tool_handle', 'tconstruct:tool_binding')):
        click(hotbar(item))
        click(300 + idx)
    click(324)
    out = g.call('menu')['carried'] or {}
    assert out.get('item') == 'tconstruct:pickaxe', out
    click(0)
    g.call('close')
    g.call('hotbar', '1')
    g.call('wait', '5')
    return g.call('inventory')['held']

def val(item, name):
    m = re.search('\\b' + name + ':(-?[0-9.]+)f?', item.get('components', ''))
    assert m, (name, item)
    return float(m.group(1))

def attack():
    cmd('kill @e[type=minecraft:zombie]')
    cmd('tp @s 0.5 71 0.5')
    cmd('gamemode survival @s')
    cmd('summon minecraft:zombie 1.5 71 0.5 {NoAI:1b}')
    g.call('wait', '30')
    g.call('hurtlog')
    entities = g.call('entities', '8', 'minecraft:zombie')['entities']
    g.call('attack', str(entities[0]['id']))
    return g.call('hurtlog')['players'][0]['attacks'][-1]['amount']
try:
    g.ready()
    if g.call('status')['screen'] == 'dead':
        g.call('respawn')
        g.call('wait', '20')
    if pack.exists():
        raise RuntimeError('test datapack path already exists')
    (pack / 'data/tconstruct/tinkering/materials/traits').mkdir(parents=True)
    (pack / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"temporary trait stat test"}}')
    created = True
    set_trait('preserved')
    preserved = build()
    print('preserved', json.dumps(preserved), flush=True)
    assert val(preserved, 'tic_max_damage') == 110
    set_trait('ductile')
    ductile = build()
    print('ductile', json.dumps(ductile), flush=True)
    assert val(ductile, 'tic_max_damage') == 97 and abs(val(ductile, 'tic_attack_damage') - 2.1875) < 0.001
    amount = attack()
    print('ductile attack', amount, flush=True)
    assert abs(amount - 3.1875) < 0.01
    cmd('gamemode creative @s')
    components = ductile['components']
    marker = '"minecraft:custom_data":'
    start = components.index(marker) + len(marker)
    depth = 0
    end = None
    for i in range(start, len(components)):
        if components[i] == '{':
            depth += 1
        elif components[i] == '}':
            depth -= 1
            if depth == 0:
                end = i + 1
                break
    custom = components[start:end]
    assert end is not None
    cmd('item replace entity @s weapon.mainhand with tconstruct:pickaxe[minecraft:damage=96,minecraft:max_damage=97,minecraft:custom_data=' + custom + ']')
    amount = attack()
    broken = g.call('inventory')['held']
    print('broken', json.dumps(broken), flush=True)
    assert broken.get('damage') == 97 and 'tic_broken:1b' in broken.get('components', '')
    cmd('gamemode creative @s')
    cmd('give @s minecraft:flint 1')
    g.call('interact', '0', '71', '2')
    g.call('wait', '10')
    click(0)
    click(300)
    click(hotbar('minecraft:flint'))
    click(301)
    click(325)
    repaired = g.call('menu')['carried'] or {}
    print('repaired', json.dumps(repaired), flush=True)
    assert repaired.get('item') == 'tconstruct:pickaxe' and repaired.get('damage', 97) < 97
    click(0)
    g.call('close')
    cmd('give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:wood"}] 1')
    g.call('interact', '0', '71', '2')
    g.call('wait', '10')
    click(0)
    click(300)
    click(hotbar('tconstruct:pick_head'))
    click(301)
    click(325)
    swapped = g.call('menu')['carried'] or {}
    assert swapped.get('item') == 'tconstruct:pickaxe', swapped
    click(0)
    g.call('screenshot', str(run / 'trait-stat-station.png'))
    g.call('close')
    held = g.call('inventory')['held']
    print('swapped', json.dumps(held), flush=True)
    assert val(held, 'tic_max_damage') == 63 and abs(val(held, 'tic_attack_damage') - 0.6) < 0.001
    g.call('save')
    g.stop()
    g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)
    g.ready()
    held = g.call('inventory')['held']
    print('saved', json.dumps(held), flush=True)
    assert val(held, 'tic_max_damage') == 63 and abs(val(held, 'tic_attack_damage') - 0.6) < 0.001
    cmd('datapack disable "file/trait-stat-test"')
    g.stop()
    print('PASS station stat traits', flush=True)
finally:
    g.cleanup()
    if created:
        (pack / 'data/tconstruct/tinkering/materials/traits/flint.json').unlink(missing_ok=True)
        (pack / 'pack.mcmeta').unlink(missing_ok=True)
        for sub in ('data/tconstruct/tinkering/materials/traits', 'data/tconstruct/tinkering/materials', 'data/tconstruct/tinkering', 'data/tconstruct', 'data', ''):
            (pack / sub).rmdir()

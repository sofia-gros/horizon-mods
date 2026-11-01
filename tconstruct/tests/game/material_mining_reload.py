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
pack = g.root / 'saves/Probe/datapacks/trait-mining-reload-test'
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

def set_trait(name, level=1):
    f = pack / 'data/tconstruct/tinkering/materials/traits/flint.json'
    f.write_text(json.dumps({'default': ([{'name': 'tconstruct:' + name, 'level': level}] if name else [])}))
    cmd('reload')
    g.call('wait', '50')

def build():
    for x in ('gamemode creative @s', 'effect clear @s', 'effect give @s minecraft:instant_health 1 4 true', 'kill @e[type=minecraft:zombie,tag=tconstruct_melee_trait_test]', 'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:tinker_station', 'clear @s', 'tp @s 0.5 71 0.5', 'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1', 'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1', 'give @s tconstruct:tool_binding[minecraft:custom_data={Material:"tconstruct:wood"}] 1'):
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


def set_pair(head, handle):
    traits = pack / 'data/tconstruct/tinkering/materials/traits'
    (traits / 'flint.json').write_text(json.dumps({'default': [{'name': 'tconstruct:' + head, 'level': 3}]}))
    (traits / 'wood.json').write_text(json.dumps({'default': [], 'perStat': {
        'tconstruct:handle': [{'name': 'tconstruct:' + handle, 'level': 3}]
    }}))
    cmd('reload'); g.call('wait', '50')

def mine(fraction=1.0, block='minecraft:stone', haste=False):
    cmd('gamemode creative @s')
    cmd('effect clear @s minecraft:haste')
    cmd('tp @s 0.5 71 0.5')
    cmd('setblock 0 71 2 ' + block)
    cmd('item modify entity @s weapon.mainhand {type:"minecraft:set_damage",function:"minecraft:set_damage",damage:' + str(fraction) + '}')
    if haste: cmd('effect give @s minecraft:haste 30 0 true')
    before = g.call('inventory')['held']
    cmd('gamemode survival @s')
    g.call('wait', '30')
    print('held before mining', json.dumps(before), flush=True)
    result = g.call('break', '0', '71', '2', '--timeout', '30', timeout=35)
    after = g.call('inventory')['held']
    print('mine', block, fraction, haste, 'damage', before.get('damage', 0), after.get('damage', 0), 'result', json.dumps(result), flush=True)
    assert result.get('block') == 'minecraft:air', result
    return result['ticks'], before.get('damage', 0), after.get('damage', 0)

def efficiency():
    cmd('item modify entity @s weapon.mainhand {type:"minecraft:set_enchantments",enchantments:{"minecraft:efficiency":2}}')
    held = g.call('inventory')['held']
    assert 'minecraft:efficiency' in held.get('components', ''), held

def repair(fraction=0.4):
    cmd('gamemode creative @s'); cmd('setblock 0 71 2 tconstruct:tinker_station')
    cmd('item modify entity @s weapon.mainhand {type:"minecraft:set_damage",function:"minecraft:set_damage",damage:' + str(fraction) + '}')
    before = (g.call('inventory')['held'] or {}).get('damage', 0)
    cmd('give @s minecraft:flint 1'); g.call('interact', '0', '71', '2'); g.call('wait', '10')
    click(0); click(300); click(hotbar('minecraft:flint')); click(301); click(325)
    result = g.call('menu')['carried'] or {}
    assert result.get('item') == 'tconstruct:pickaxe', result
    repaired = result.get('damage', 0)
    g.call('screenshot', str(run / 'material-mining-repair.png'))
    click(0); g.call('close')
    print('repair', before, repaired, flush=True)
    return before - repaired

try:
    g.ready()
    if g.call('status')['screen'] == 'dead': g.call('respawn'); g.call('wait', '20')
    if pack.exists(): raise RuntimeError('test datapack path already exists')
    (pack / 'data/tconstruct/tinkering/materials/traits').mkdir(parents=True)
    (pack / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"temporary mining reload test"}}')
    created = True
    cmd('reload'); cmd('datapack enable "file/trait-mining-reload-test"'); g.call('wait', '50')
    set_trait('jagged', 3); build(); mine(0.7); before = mine(0.7)[0]
    g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    held = g.call('inventory')['held']
    assert 'tic_mining_tool' in held.get('components', '') and 'tconstruct:jagged' in held.get('components', ''), held
    mine(0.7); after = mine(0.7)[0]
    print('before/after reload mining ticks', before, after, flush=True)
    assert before == after, (before, after)
    cmd('datapack disable "file/trait-mining-reload-test"'); g.stop()
    print('PASS material mining save and reload', flush=True)
finally:
    g.cleanup()
    if created:
        (pack / 'data/tconstruct/tinkering/materials/traits/flint.json').unlink(missing_ok=True)
        (pack / 'pack.mcmeta').unlink(missing_ok=True)
        for sub in ('data/tconstruct/tinkering/materials/traits', 'data/tconstruct/tinkering/materials', 'data/tconstruct/tinkering', 'data/tconstruct', 'data', ''): (pack / sub).rmdir()

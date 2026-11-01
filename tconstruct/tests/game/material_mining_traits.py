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
pack = g.root / 'saves/Probe/datapacks/trait-mining-followup-test'
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
    (pack / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"temporary mining trait test"}}')
    created = True
    cmd('reload'); cmd('datapack enable "file/trait-mining-followup-test"'); g.call('wait', '50')
    set_trait(''); build(); mine(); baseline = mine()[0]
    set_trait('lightweight', 2); build(); lightweight = mine()[0]
    set_trait('heavy', 2); build(); heavy = mine()[0]
    set_trait('vintage', 2); build(); vintage = mine()[0]
    set_trait('entangled', 1); build(); entangled = mine()[0]
    print('static ticks', baseline, lightweight, heavy, vintage, entangled, flush=True)
    assert lightweight < baseline and heavy < baseline and vintage > baseline and entangled < baseline
    set_trait('jagged', 3); build(); fresh = mine(1.0)[0]; worn = mine(0.7)[0]
    set_trait('stonebound', 3); build(); stone_fresh = mine(1.0)[0]; stone_worn = mine(0.7)[0]
    set_trait('maintained'); build(); maintained_fresh = mine(1.0)[0]; maintained_worn = mine(0.2)[0]
    print('conditional ticks', fresh, worn, stone_fresh, stone_worn, maintained_fresh, maintained_worn, flush=True)
    assert worn > fresh and stone_worn < stone_fresh and maintained_worn > maintained_fresh
    # The conditional percentages run after mining efficiency and Haste.
    set_trait(''); build(); efficiency(); enchanted_base = mine(0.7, haste=True)[0]
    set_trait('jagged', 3); build(); efficiency(); enchanted_jagged = mine(0.7, haste=True)[0]
    print('efficiency and haste ticks', enchanted_base, enchanted_jagged, flush=True)
    assert enchanted_jagged > enchanted_base
    for other in ('jagged', 'stonebound'):
        set_pair(other, 'maintained'); build(); first = mine(0.7, 'minecraft:iron_block')[0]
        set_pair('maintained', other); build(); second = mine(0.7, 'minecraft:iron_block')[0]
        print('modifier priority', other, first, second, flush=True)
        assert first == second, (other, first, second)
    (pack / 'data/tconstruct/tinkering/materials/traits/wood.json').unlink()
    set_trait(''); build(); ineffective_base = mine(block='minecraft:dirt')[0]
    set_trait('entangled'); build(); ineffective_trait = mine(block='minecraft:dirt')[0]
    assert ineffective_base == ineffective_trait, (ineffective_base, ineffective_trait)
    set_trait(''); build(); ordinary_repair = repair()
    set_trait('dense'); build(); dense_repair = repair()
    assert 0 < dense_repair < ordinary_repair, (ordinary_repair, dense_repair)
    cmd('gamemode creative @s'); cmd('clear @s'); cmd('give @s minecraft:iron_pickaxe 1'); g.call('hotbar', '1')
    vanilla_ticks = mine()[0]
    assert vanilla_ticks < 20, vanilla_ticks
    print('vanilla iron pickaxe ticks', vanilla_ticks, flush=True)
    g.call('screenshot', str(run / 'material-mining-traits.png'))
    cmd('datapack disable "file/trait-mining-followup-test"'); g.stop()
    print('PASS mining trait source values and Dense repair', flush=True)
finally:
    g.cleanup()
    if created:
        (pack / 'data/tconstruct/tinkering/materials/traits/flint.json').unlink(missing_ok=True)
        (pack / 'data/tconstruct/tinkering/materials/traits/wood.json').unlink(missing_ok=True)
        (pack / 'pack.mcmeta').unlink(missing_ok=True)
        for sub in ('data/tconstruct/tinkering/materials/traits', 'data/tconstruct/tinkering/materials', 'data/tconstruct/tinkering', 'data/tconstruct', 'data', ''): (pack / sub).rmdir()

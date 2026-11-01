import argparse, json, re, sys, time
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
for k in ('game', 'ctl', 'resources'):
    p.add_argument('--' + k, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--in-frame', action='store_true')
p.add_argument('--only-transaction', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)
pack = g.root / 'saves/Probe/datapacks/trait-melee-followup-test'
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

def attack():
    cmd('kill @e[type=minecraft:zombie,tag=tconstruct_melee_trait_test]')
    cmd('tp @s 0.5 71 0.5')
    cmd('gamemode survival @s')
    cmd('summon minecraft:zombie 1.5 71 0.5 {NoAI:1b,Tags:["tconstruct_melee_trait_test"]}')
    g.call('wait', '30')
    g.call('hurtlog')
    entities = g.call('entities', '8', 'minecraft:zombie')['entities']
    g.call('attack', str(entities[0]['id']))
    return g.call('hurtlog')['players'][0]['attacks'][-1]['amount']
def cooldown_scale():
    cmd('kill @e[type=minecraft:zombie,tag=tconstruct_melee_trait_test]')
    cmd('tp @s 0.5 71 0.5')
    cmd('gamemode survival @s')
    cmd('summon minecraft:zombie 1.5 71 0.5 {NoAI:1b,Tags:["tconstruct_melee_trait_test"]}')
    g.call('wait', '35'); g.call('hurtlog')
    target = g.call('entities', '8', 'minecraft:zombie')['entities'][0]['id']
    g.call('attack', str(target)); g.call('wait', '5'); g.call('attack', str(target))
    result = g.call('hurtlog')['players'][0]['attacks'][-1]['scale']
    cmd('gamemode creative @s')
    return result

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
    cmd('reload')
    cmd('datapack enable "file/trait-melee-followup-test"')
    g.call('wait', '50')
    if not a.only_transaction:
        set_trait('')
        baseline = build()
        base_damage = val(baseline, 'tic_attack_damage')
        base_speed = val(baseline, 'tic_attack_speed')
        assert abs(base_damage - 1.75) < 0.001 and abs(base_speed - 1.2) < 0.001
        baseline_scale = cooldown_scale()
        for name, level, expected_damage, expected_speed in [
            ('solid', 1, base_damage * 1.15, base_speed),
            ('pierce', 2, base_damage + 1.0, base_speed),
            ('lightweight', 2, base_damage, base_speed * 1.16),
            ('vintage', 1, base_damage, base_speed * 0.9),
        ]:
            set_trait(name, level)
            tool = build()
            print(name, json.dumps(tool), flush=True)
            assert abs(val(tool, 'tic_attack_damage') - expected_damage) < 0.001
            assert abs(val(tool, 'tic_attack_speed') - expected_speed) < 0.001
            if name in ('lightweight', 'vintage'):
                scale = cooldown_scale()
                print('cooldown', name, baseline_scale, scale, flush=True)
                assert scale > baseline_scale if name == 'lightweight' else scale < baseline_scale
            if name in ('solid', 'pierce'):
                amount = attack()
                assert abs(amount - (1.0 + expected_damage)) < 0.01, amount
                cmd('gamemode creative @s')
    # Upgrade, swap and save with a material-derived Solid trait.
    set_trait('solid')
    build()
    cmd('give @s minecraft:quartz 36')
    g.call('interact', '0', '71', '2'); g.call('wait', '10')
    click(0); click(300); click(hotbar('minecraft:quartz')); click(301); click(325)
    assert (g.call('menu')['carried'] or {}).get('item') == 'tconstruct:pickaxe'
    click(0); g.call('close')
    upgraded = g.call('inventory')['held']
    assert abs(val(upgraded, 'tic_attack_damage') - (1.75 + 0.75) * 1.15) < 0.001, upgraded
    cmd('give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:wood"}] 1')
    g.call('interact', '0', '71', '2'); g.call('wait', '10')
    click(0); click(300); click(hotbar('tconstruct:pick_head')); click(301); click(325)
    assert (g.call('menu')['carried'] or {}).get('item') == 'tconstruct:pickaxe'
    click(0); g.call('screenshot', str(run / 'material-melee-followup.png')); g.call('close')
    before = g.call('inventory')['held']
    assert 'tic_upgrades' in before.get('components', '')
    g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    saved = g.call('inventory')['held']
    assert abs(val(saved, 'tic_attack_damage') - val(before, 'tic_attack_damage')) < 0.001
    assert abs(val(saved, 'tic_attack_speed') - val(before, 'tic_attack_speed')) < 0.001
    cmd('datapack disable "file/trait-melee-followup-test"')
    g.stop()
    print('PASS source melee trait stats, upgrade, swap and save', flush=True)
finally:
    g.cleanup()
    if created:
        (pack / 'data/tconstruct/tinkering/materials/traits/flint.json').unlink(missing_ok=True)
        (pack / 'pack.mcmeta').unlink(missing_ok=True)
        for sub in ('data/tconstruct/tinkering/materials/traits', 'data/tconstruct/tinkering/materials', 'data/tconstruct/tinkering', 'data/tconstruct', 'data', ''):
            (pack / sub).rmdir()

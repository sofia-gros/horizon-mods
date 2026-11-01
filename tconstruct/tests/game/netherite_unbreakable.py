"""Exercise source slot bonuses and the complete modifier transaction chain."""
import argparse
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for name in ('game', 'ctl', 'resources', 'run-dir'):
    p.add_argument('--' + name, type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2] / 'target').resolve())
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)

def cmd(s):
    g.call('command', s); g.call('wait', '7')
def click(slot, *args):
    g.call('click-slot', str(slot), *args); g.call('wait', '5')
def hot(item):
    return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item') == item)
def slots():
    return {x['slot']: x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
def current():
    return next(x for x in g.call('inventory')['hotbar'] if x.get('item') == 'tconstruct:pickaxe')
def apply(table, ingredient_slots, expected, reject=False):
    cmd('setblock 0 71 2 tconstruct:' + table)
    g.call('interact', '0', '71', '2'); g.call('wait', '12')
    click(hot('tconstruct:pickaxe')); click(300)
    # Each item is placed in a different workstation slot, as in the source.
    for item, targets in ingredient_slots:
        click(hot(item))
        for target in targets: click(target, 'right')
        if g.call('menu').get('carried'): click(hot_empty())
    g.call('wait', '8')
    if reject:
        assert 325 not in slots(), (expected, slots())
        g.call('close'); g.call('wait', '8'); return
    assert 325 in slots(), (expected, slots())
    if expected == 'unbreakable': g.call('screenshot', str(run / 'netherite-unbreakable.png'))
    click(325)
    result = g.call('menu').get('carried')
    assert result and result['item'] == 'tconstruct:pickaxe', (expected, result, slots())
    assert f'"tconstruct:{expected}"' in result.get('components', ''), result
    if expected == 'unbreakable':
        assert slots().get(302, {}).get('item') == 'minecraft:glass_bottle', slots()
    click(0); g.call('close'); g.call('wait', '8')
    print('APPLIED', expected, current(), flush=True)
def hot_empty():
    inv = g.call('inventory')['hotbar']
    return next(x['slot'] for x in inv if not x.get('item'))

try:
    g.ready()
    if g.call('status')['screen'] == 'dead': g.call('respawn'); g.call('wait', '20')
    for command in ('gamemode creative @s', 'time set day', 'clear @s', 'tp @s 0.5 71 0.5',
                    'fill -2 71 -1 2 72 3 minecraft:air',
                    'give @s tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85},minecraft:max_damage=85] 1'):
        cmd(command)
    for modifier, item in [('writable','minecraft:writable_book'), ('harmonious','minecraft:music_disc_13'), ('recapitated','minecraft:skeleton_skull'), ('forecast','minecraft:diamond_ore')]:
        cmd('give @s ' + item + ' 1')
        apply('tinker_station', [(item, [301])], modifier)
    cmd('give @s minecraft:netherite_upgrade_smithing_template 1')
    cmd('give @s minecraft:netherite_ingot 1')
    apply('tinker_station', [('minecraft:netherite_upgrade_smithing_template',[301]), ('minecraft:netherite_ingot',[302])], 'netherite')
    for _ in range(5):
        cmd('give @s tconstruct:emerald_reinforcement 4')
        # Incremental recipes consume up to the next complete level.
        cmd('setblock 0 71 2 tconstruct:tinker_station')
        g.call('interact','0','71','2'); g.call('wait','12')
        click(hot('tconstruct:pickaxe')); click(300)
        click(hot('tconstruct:emerald_reinforcement')); click(301)
        assert 325 in slots(), slots()
        click(325); result=g.call('menu')['carried']; assert result and result['item']=='tconstruct:pickaxe',result
        click(0); g.call('close'); g.call('wait','8')
    assert '"tconstruct:reinforced":20' in current().get('components',''), current()
    for command in ('give @s minecraft:shulker_shell 2','give @s minecraft:dragon_breath 1','give @s minecraft:netherite_ingot 2'): cmd(command)
    apply('tinkers_anvil', [('minecraft:shulker_shell',[301,303]),('minecraft:dragon_breath',[302]),('minecraft:netherite_ingot',[304,305])], 'unbreakable')
    assert '"tconstruct:unbreakable":1' in current().get('components',''), current()
    print('PASS natural recipe transaction chain using supplied ingredients', flush=True)
    g.call('save'); g.stop()
finally:
    g.cleanup()

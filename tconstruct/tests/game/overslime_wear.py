"""Representative shield wear on supplied tools and two armor pieces."""
import argparse
import re
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for name in ('game', 'ctl', 'resources', 'run-dir'): p.add_argument('--' + name, type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args(); run = a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2] / 'target').resolve())
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)
def cmd(value): g.call('command', value); g.call('wait', '7')
def custom(parts):
    return ('minecraft:max_damage=100,minecraft:custom_data={tic_materials:' + parts +
            ',tic_max_damage:100,tic_draw_speed:2.0f,tic_velocity:1.0f,tic_accuracy:1.0f,tic_overslime:20,tic_overslime_capacity:50,tic_overslime_active:1b,'
            'tic_upgrades:{"tconstruct:overslime":1,"tconstruct:unbreakable":1}}')
def shield(stack):
    return re.search(r'tic_overslime["\']?\s*:\s*19\b', str(stack)) is not None
try:
    g.ready()
    if g.call('status')['screen'] == 'dead': g.call('respawn'); g.call('wait', '20')
    pick = 'tconstruct:pickaxe[' + custom('["tconstruct:flint","tconstruct:wood","tconstruct:wood"]') + ']'
    for value in ('gamemode creative @s', 'clear @s', 'effect clear @s', 'tp @s 0.5 71 0.5',
                  'setblock 0 71 2 minecraft:stone', 'item replace entity @s weapon.mainhand with ' + pick,
                  'gamemode survival @s'): cmd(value)
    g.call('break', '0', '71', '2', '--timeout', '20', timeout=25)
    held = g.call('inventory')['held']; print('OVERSLIME PICK', held, flush=True)
    assert held.get('damage', 0) == 0 and shield(held), held
    for kind, parts in [('longbow','["tconstruct:wood","tconstruct:wood","tconstruct:wood","tconstruct:string"]'),
                        ('crossbow','["tconstruct:wood","tconstruct:wood","tconstruct:string"]')]:
        bow = 'tconstruct:' + kind + '[' + custom(parts) + ']'
        g.call('hotbar','1');g.call('wait','5')
        for value in ('gamemode creative @s', 'clear @s', 'item replace entity @s weapon.mainhand with ' + bow,
                      'give @s minecraft:arrow 4', 'gamemode survival @s'): cmd(value)
        g.call('hotbar','1');g.call('look','0','-60');g.call('wait','10')
        g.call('mouse','right','down');g.call('wait','35');g.call('mouse','right','up')
        if kind == 'crossbow':
            g.call('wait','4');g.call('mouse','right','down');g.call('wait','2');g.call('mouse','right','up')
        g.call('wait','10');held=g.call('inventory')['held'];print('OVERSLIME BOW',kind,held,flush=True)
        assert held.get('damage',0)==0 and shield(held),held
    helmet = 'tconstruct:plate_helmet[' + custom('["tconstruct:iron","tconstruct:iron"]') + ']'
    chest = 'tconstruct:plate_chestplate[' + custom('["tconstruct:iron","tconstruct:iron"]') + ']'
    for value in ('gamemode creative @s', 'effect give @s minecraft:instant_health 1 4 true',
                  'item replace entity @s armor.head with ' + helmet, 'item replace entity @s armor.chest with ' + chest,
                  'gamemode survival @s', 'damage @s 4 minecraft:mob_attack'): cmd(value)
    armor = g.call('inventory')['armor']; print('OVERSLIME ARMOR', armor, flush=True)
    for stack in armor[:2]:
        assert stack and stack.get('damage', 0) == 0 and shield(stack), armor
    g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    armor = g.call('inventory')['armor']
    for stack in armor[:2]: assert stack and shield(stack), armor
    g.stop(); print('PASS supplied shield before Unbreakable and armor reload', flush=True)
finally:
    g.cleanup()

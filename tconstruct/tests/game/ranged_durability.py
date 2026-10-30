"""Exercise material bow wear, broken retention and limb-material repair."""
import argparse
import json
import re
from pathlib import Path
from harness import Game

parser = argparse.ArgumentParser()
for name in ('game', 'ctl', 'resources'):
    parser.add_argument('--' + name, type=Path, required=True)
parser.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
args = parser.parse_args()
run = args.run_dir.resolve()
game = Game(args, run, 'Player', resume=True)

def cmd(text):
    result = game.call('command', text)
    game.call('wait', '8')
    return result

def click(slot):
    game.call('click-slot', str(slot))
    game.call('wait', '8')

def shoot(kind):
    if kind == 'longbow':
        game.call('mouse', 'right', 'down')
        game.call('wait', '30')
        game.call('mouse', 'right', 'up')
        game.call('wait', '12')
    else:
        game.call('mouse', 'right', 'down')
        game.call('wait', '45')
        game.call('mouse', 'right', 'up')
        game.call('wait', '8')
        game.call('mouse', 'right', 'down')
        game.call('wait', '2')
        game.call('mouse', 'right', 'up')
        game.call('wait', '12')

def held():
    return game.call('inventory')['held']

def arrows():
    return game.call('inventory')['counts'].get('minecraft:arrow', 0)

def damage(stack):
    match = re.search(r'"minecraft:damage":(\d+)', stack.get('components', ''))
    return int(match.group(1)) if match else 0

try:
    game.ready()
    if game.call('status')['screen'] == 'dead':
        game.call('respawn')
        game.call('wait', '20')
    for kind in ('longbow', 'crossbow'):
        game.call('hotbar', '1')
        materials = '["tconstruct:wood","tconstruct:wood","tconstruct:string"]'
        if kind == 'longbow':
            materials = '["tconstruct:wood","tconstruct:wood","tconstruct:wood","tconstruct:string"]'
        weapon = ('tconstruct:' + kind + '[minecraft:custom_data={tic_materials:' + materials
                  + ',tic_draw_speed:2.0f,tic_velocity:1.0f,tic_accuracy:1.0f,tic_max_damage:60},'
                  + 'minecraft:max_damage=60,minecraft:damage=58]')
        for text in ('gamemode creative @s', 'effect clear @s', 'clear @s', 'tp @s 0.5 71 0.5',
                     'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:tinker_station',
                     'item replace entity @s weapon.mainhand with ' + weapon, 'give @s minecraft:arrow 8'):
            cmd(text)
        before = arrows()
        shoot(kind)
        assert damage(held()) == 58 and arrows() == before, (kind, 'creative', held(), before, arrows())
        cmd('gamemode survival @s')
        for expected in (59, 60):
            before = arrows()
            shoot(kind)
            game.call('wait', '20')
            item = held()
            assert item['item'] == 'tconstruct:' + kind and damage(item) == expected, (kind, expected, item)
            assert arrows() == before - 1, (kind, before, arrows())
        assert 'tic_broken:1b' in held().get('components', ''), held()
        before = arrows()
        shoot(kind)
        assert arrows() == before and damage(held()) == 60, (kind, 'broken use', held())
        cmd('give @s minecraft:oak_planks 1')
        game.call('interact', '0', '71', '2')
        game.call('wait', '20')
        click(0)
        click(300)
        slot = next(s['slot'] for s in game.call('inventory')['hotbar'] if s.get('item') == 'minecraft:oak_planks')
        click(slot)
        click(301)
        click(325)
        repaired = game.call('menu')['carried'] or {}
        assert repaired.get('item') == 'tconstruct:' + kind and repaired.get('damage', 60) < 60, repaired
        click(0)
        game.call('close')
        game.call('wait', '20')
        assert 'tic_broken:0b' in held().get('components', ''), held()
        game.call('save')
        game.stop()
        game.cleanup()
        game = Game(args, run, 'Player', resume=True)
        game.ready()
        assert held()['item'] == 'tconstruct:' + kind and damage(held()) < 60, held()
        print('PASS', kind, 'creative exemption, survival wear, retention, repair and reload', flush=True)
    game.stop()
finally:
    game.cleanup()

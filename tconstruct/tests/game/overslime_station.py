"""Exercise a supplied pickaxe and source slime through the Station and crafting grid."""
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

def cmd(value):
    g.call('command', value); g.call('wait', '8')
def click(slot):
    g.call('click-slot', str(slot)); g.call('wait', '8')
def hot(item):
    return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item') == item)
def slots():
    return {x['slot']: x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}

try:
    g.ready()
    if g.call('status')['screen'] == 'dead': g.call('respawn'); g.call('wait', '20')
    for value in ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5',
                  'fill -2 71 -1 2 72 3 minecraft:air', 'setblock 0 71 2 tconstruct:tinker_station',
                  'give @s tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85},minecraft:max_damage=85] 1',
                  'give @s minecraft:slime_ball 1', 'give @s tconstruct:slimesteel_reinforcement 1'):
        cmd(value)
    g.call('interact', '0', '71', '2'); g.call('wait', '12')
    for item, slot in [('tconstruct:pickaxe', 300), ('tconstruct:slimesteel_reinforcement', 301)]:
        click(hot(item)); click(slot)
    click(325); item = g.call('menu')['carried']; assert item and item['item'] == 'tconstruct:pickaxe', slots()
    click(0); g.call('close'); g.call('wait', '8')
    g.call('interact', '0', '71', '2'); g.call('wait', '12')
    for name, slot in [('tconstruct:pickaxe', 300), ('minecraft:slime_ball', 301)]:
        click(hot(name)); click(slot)
    g.call('screenshot', str(run / 'overslime-station.png'))
    click(325); item = g.call('menu')['carried']; assert item and item['item'] == 'tconstruct:pickaxe', slots()
    click(0); g.call('close'); g.call('wait', '8')
    inventory = g.call('inventory'); print('OVERSLIME OUTPUT', inventory, flush=True)
    item = next(stack for stack in inventory['hotbar'] if stack.get('item') == 'tconstruct:pickaxe')
    assert 'tic_overslime:20' in item.get('components', '') and 'tic_overslime_capacity:68' in item.get('components', ''), item
    g.call('save'); g.stop()
    print('PASS fixture Station Overforced and Overslime', flush=True)
finally:
    g.cleanup()

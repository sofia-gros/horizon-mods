"""Exercise source Melter fluid and fuel tank clicks in the shared world."""
import argparse
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for key in ('game', 'ctl', 'resources'):
    p.add_argument('--' + key, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)

def click(slot):
    g.call('click-slot', str(slot)); g.call('wait', '5')
def hotbar(item):
    return next(slot['slot'] for slot in g.call('inventory')['hotbar'] if slot.get('item') == item)
def carried():
    return (g.call('menu').get('carried') or {}).get('item')
def tank(x, y, right=False):
    g.call('click-gui', str(x), str(y), *(['right'] if right else [])); g.call('wait', '10')

try:
    g.ready()
    for command in ('gamemode creative @s', 'clear @s', 'tp @s 2.5 71 1.5',
                    'setblock 0 72 2 minecraft:air', 'setblock 0 71 2 minecraft:air',
                    'setblock 0 71 2 tconstruct:seared_fuel_tank', 'setblock 0 72 2 tconstruct:seared_melter',
                    'give @s minecraft:lava_bucket 1', 'give @s tconstruct:molten_copper_bucket 1',
                    'give @s minecraft:copper_ingot 1'):
        g.call('command', command)
    g.call('wait', '10'); g.call('interact', '0', '72', '2'); g.call('wait', '15')
    click(hotbar('minecraft:lava_bucket')); tank(158, 50, True)
    assert carried() == 'minecraft:bucket', 'fuel deposit did not return empty bucket'
    tank(158, 50)
    assert carried() == 'minecraft:lava_bucket', 'fuel extraction failed'
    tank(158, 50, True)
    assert carried() == 'minecraft:bucket', 'fuel deposit retry failed'
    empty = next(slot['slot'] for slot in g.call('inventory')['hotbar'] if not slot.get('item'))
    click(empty); click(hotbar('tconstruct:molten_copper_bucket')); tank(100, 45, True)
    assert carried() == 'minecraft:bucket', 'output tank deposit failed'
    tank(100, 45)
    assert carried() == 'tconstruct:molten_copper_bucket', 'output tank extraction failed'
    empty = next(slot['slot'] for slot in g.call('inventory')['hotbar'] if not slot.get('item'))
    click(empty); click(hotbar('minecraft:copper_ingot')); click(300)
    g.call('wait', '500', timeout=55)
    assert not any(w.get('id') == 'slot-300' and w.get('stack') for w in g.call('ui')['widgets']), 'liquid fuel did not melt copper'
    g.call('screenshot', str(run / 'melter-fluid-fuel.png'))
    g.call('close'); g.call('save'); g.stop()
    print('PASS Melter output and fuel tank clicks and liquid-fuel melting', flush=True)
finally:
    g.cleanup()

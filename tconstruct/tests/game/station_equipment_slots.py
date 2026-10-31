"""Exercise original Tinker Station armor and offhand slots."""
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

def click(slot, *flags):
    g.call('click-slot', str(slot), *flags); g.call('wait', '5')

def hotbar(item):
    return next(slot['slot'] for slot in g.call('inventory')['hotbar'] if slot.get('item') == item)

try:
    g.ready()
    for command in ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5',
                    'setblock -1 71 1 minecraft:air', 'setblock 0 71 1 minecraft:air', 'setblock 1 71 1 minecraft:air',
                    'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:tinker_station',
                    'give @s minecraft:iron_helmet 1', 'give @s minecraft:stone 1'):
        g.call('command', command)
    g.call('wait', '15'); g.call('interact', '0', '71', '2'); g.call('wait', '10')
    click(hotbar('minecraft:iron_helmet')); click(500)
    assert g.call('inventory')['armor'][0].get('item') == 'minecraft:iron_helmet', 'helmet was not equipped'
    click(500, 'shift')
    assert not g.call('inventory')['armor'][0].get('item'), 'shift move did not unequip helmet'
    click(hotbar('minecraft:stone')); click(504)
    assert any(row.get('id') == 'slot-504' and (row.get('stack') or {}).get('item') == 'minecraft:stone' for row in g.call('ui')['widgets']), 'offhand slot did not update'
    click(504); assert (g.call('menu').get('carried') or {}).get('item') == 'minecraft:stone'
    g.call('screenshot', str(run / 'station-equipment-slots.png'))
    g.call('close'); g.call('save'); g.stop()
    print('PASS Station armor and offhand slots', flush=True)
finally:
    g.cleanup()

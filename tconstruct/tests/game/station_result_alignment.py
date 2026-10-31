"""Capture and exercise the source Station result frame for both operations."""
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

try:
    g.ready()
    for command in ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5',
                    'setblock -1 71 1 minecraft:air', 'setblock 0 71 1 minecraft:air', 'setblock 1 71 1 minecraft:air',
                    'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:tinker_station',
                    'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
                    'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1',
                    'give @s tconstruct:tool_binding[minecraft:custom_data={Material:"tconstruct:wood"}] 1',
                    'give @s minecraft:quartz 35'):
        g.call('command', command)
    g.call('wait', '15'); g.call('interact', '0', '71', '2'); g.call('wait', '10')
    for index, item in enumerate(('tconstruct:pick_head', 'tconstruct:tool_handle', 'tconstruct:tool_binding')):
        click(hotbar(item)); click(300 + index)
    g.call('screenshot', str(run / 'station-result-build.png'))
    click(324)
    assert (g.call('menu').get('carried') or {}).get('item') == 'tconstruct:pickaxe', 'building result missing'
    empty = next(slot['slot'] for slot in g.call('inventory')['hotbar'] if not slot.get('item'))
    click(empty); g.call('close'); g.call('wait', '5'); g.call('interact', '0', '71', '2'); g.call('wait', '10')
    click(hotbar('tconstruct:pickaxe')); click(300); click(hotbar('minecraft:quartz')); click(301)
    g.call('screenshot', str(run / 'station-result-upgrade.png'))
    click(325)
    assert (g.call('menu').get('carried') or {}).get('item') == 'tconstruct:pickaxe', 'upgrade result missing'
    g.call('close'); g.call('save'); g.stop()
    print('PASS build and upgrade result clicks; compare saved screenshots with source frame', flush=True)
finally:
    g.cleanup()

"""Exercise adjacent source workstation tabs using the shared test world."""
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


def widget(slot):
    return any(row.get('id') == f'slot-{slot}' for row in g.call('ui')['widgets'])

try:
    g.ready()
    for command in ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5',
                    'setblock -1 71 1 minecraft:air', 'setblock 0 71 1 minecraft:air', 'setblock 1 71 1 minecraft:air',
                    'setblock 0 71 2 minecraft:air', 'setblock 1 71 2 minecraft:air',
                    'setblock 0 71 2 tconstruct:part_builder', 'setblock 1 71 2 tconstruct:tinker_station',
                    'give @s minecraft:stone 1'):
        g.call('command', command)
    g.call('wait', '15'); g.call('interact', '0', '71', '2'); g.call('wait', '10')
    assert widget(322), 'Part Builder did not open'
    hotbar = next(slot['slot'] for slot in g.call('inventory')['hotbar'] if slot.get('item') == 'minecraft:stone')
    g.call('click-slot', str(hotbar)); g.call('wait', '5')
    g.call('click-gui', '45', '-15'); g.call('wait', '15')
    assert widget(324) or widget(325), 'Station tab did not open'
    assert (g.call('menu').get('carried') or {}).get('item') == 'minecraft:stone', 'cursor item was lost while switching'
    g.call('screenshot', str(run / 'workstation-tabs-station.png'))
    g.call('click-gui', '17', '-19'); g.call('wait', '15')
    assert widget(322), 'Part Builder tab did not reopen'
    assert (g.call('menu').get('carried') or {}).get('item') == 'minecraft:stone', 'cursor item was lost switching back'
    g.call('close'); g.call('save'); g.stop()
    print('PASS workstation tabs and carried item', flush=True)
finally:
    g.cleanup()

"""Capture each steel Plate armor slot in one Station session.

The armor stacks use synthetic NBT to focus on preview rendering.
"""
import argparse
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for key in ('game', 'ctl', 'resources'):
    p.add_argument('--' + key, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
a = p.parse_args(); run = a.run_dir.resolve()
game = Game(a, run, 'Player', resume=True)
try:
    game.ready()
    for command in ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5',
                    'setblock 0 71 1 minecraft:air', 'setblock 0 71 2 minecraft:air',
                    'setblock 0 71 2 tconstruct:tinker_station'):
        game.call('command', command)
    for part in ('helmet', 'chestplate', 'leggings', 'boots'):
        game.call('command', f'give @s tconstruct:plate_{part}[minecraft:max_damage=382,minecraft:custom_data={{tic_max_damage:382,tic_materials:["tconstruct:steel","tconstruct:steel"]}}] 1')
    game.call('wait', '15'); game.call('interact', '0', '71', '2'); game.call('wait', '15')
    game.call('click-gui', '-121', '18'); game.call('wait', '10')
    for part in ('helmet', 'chestplate', 'leggings', 'boots'):
        item = 'tconstruct:plate_' + part
        slot = next(x['slot'] for x in game.call('inventory')['hotbar'] if x.get('item') == item)
        game.call('click-slot', str(slot)); game.call('click-slot', '300'); game.call('wait', '20')
        current = next(x for x in game.call('menu')['slots'] if x['slot'] == 300)
        assert (current.get('stack') or {}).get('item') == item, current
        game.call('screenshot', str(run / ('plate-preview-' + part + '.png')))
        if part == 'chestplate':
            game.call('drag-at', '95', '200', '110', '200'); game.call('wait', '10')
            game.call('screenshot', str(run / 'plate-preview-rotated.png'))
        game.call('click-slot', '300'); game.call('click-slot', str(slot)); game.call('wait', '10')
    game.call('close'); game.call('save'); game.stop()
    print('PASS four synthetic steel plate previews; inspect screenshots', flush=True)
finally:
    game.cleanup()

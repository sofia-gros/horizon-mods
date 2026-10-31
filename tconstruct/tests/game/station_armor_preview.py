"""Capture the Station preview while a real dagger result is available."""
import argparse
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for key in ('game', 'ctl', 'resources'):
    p.add_argument('--' + key, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
a = p.parse_args()
run = a.run_dir.resolve()
game = Game(a, run, 'Player', resume=True)
try:
    game.ready()
    for command in ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5',
                    'setblock 0 71 1 minecraft:air', 'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:tinker_station',
                    'give @s tconstruct:small_blade[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
                    'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1'):
        game.call('command', command)
    game.call('wait', '15'); game.call('interact', '0', '71', '2'); game.call('wait', '15')
    game.call('click-gui', '-99', '18'); game.call('wait', '10')
    for item, target in (('tconstruct:small_blade', 300), ('tconstruct:tool_handle', 301)):
        source = next(row['slot'] for row in game.call('inventory')['hotbar'] if row.get('item') == item)
        game.call('click-slot', str(source)); game.call('wait', '5')
        game.call('click-slot', str(target)); game.call('wait', '10')
    result = next(row for row in game.call('menu')['slots'] if row['slot'] == 324)
    assert (result.get('stack') or {}).get('item') == 'tconstruct:dagger', result
    game.call('wait', '220')
    game.call('screenshot', str(run / 'station-armor-preview.png'))
    # Rotate using the same physical position as the visible stand.
    size = __import__('os').environ.get('HORIZON_TEST_SIZE', '960x540')
    if size == '960x540':
        game.call('drag-at', '95', '200', '110', '200'); game.call('wait', '10')
        game.call('screenshot', str(run / 'station-armor-rotated.png'))
    game.call('close'); game.call('save'); game.stop()
    print('PASS dagger result; inspect armor stand screenshot', flush=True)
finally:
    game.cleanup()

"""Check workstation render data and capture each table beside a solid block."""
import argparse
import json
from pathlib import Path
from harness import Game

parser = argparse.ArgumentParser()
for name in ('game', 'ctl', 'resources'):
    parser.add_argument('--' + name, type=Path, required=True)
parser.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
args = parser.parse_args()
root = args.resources
facts = json.loads((root / 'facts.json').read_text())
gameplay = json.loads((root / 'gameplay-facts.json').read_text())['states']
render = json.loads((root / 'render-definitions.json').read_text())
for name in ('tconstruct:part_builder', 'tconstruct:tinker_station'):
    row = gameplay[facts['defaults'][name]]
    assert row['solid_render'] is False and row['can_occlude'] is False
    assert render[name]['can_occlude'] is False
stone = gameplay[facts['defaults']['minecraft:stone']]
assert stone['solid_render'] is True and render['minecraft:stone']['can_occlude'] is True
run = args.run_dir.resolve()
game = Game(args, run, 'Player', resume=True)
try:
    game.ready()
    for command in ('gamemode creative @s', 'time set day', 'tp @s 0.5 71 -1.5',
                    'setblock 0 70 2 minecraft:stone', 'setblock 1 71 2 minecraft:stone'):
        game.call('command', command)
        game.call('wait', '10')
    for name in ('part_builder', 'tinker_station'):
        game.call('command', 'setblock 0 71 2 minecraft:air')
        game.call('command', 'setblock 0 71 2 tconstruct:' + name)
        game.call('wait', '20')
        assert game.call('block', '0', '71', '2')['block'] == 'tconstruct:' + name
        game.call('look-at', 'block', '0', '71', '2')
        game.call('wait', '60')
        game.call('screenshot', str(run / ('occlusion-' + name + '.png')))
    game.stop()
finally:
    game.cleanup()
print('Check the screenshots: the ground and adjacent stone face must remain visible.')

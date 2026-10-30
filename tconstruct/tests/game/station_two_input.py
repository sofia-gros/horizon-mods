"""Build a two-part tool using the source Station layout."""
import argparse
from pathlib import Path
from harness import Game
from x11_text import type_text

parser = argparse.ArgumentParser()
for name in ('game', 'ctl', 'resources'):
    parser.add_argument('--' + name, type=Path, required=True)
parser.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
parser.add_argument('--in-frame', action='store_true')
args = parser.parse_args()
run = args.run_dir.resolve()
game = Game(args, run, 'Player', in_frame=args.in_frame, resume=True)

def cmd(text):
    game.call('command', text)
    game.call('wait', '8')

def click(slot):
    game.call('click-slot', str(slot))
    game.call('wait', '8')

def hotbar(item):
    return next(s['slot'] for s in game.call('inventory')['hotbar'] if s.get('item') == item)

try:
    game.ready()
    for text in ('gamemode creative @s', 'kill @e[type=minecraft:zombie,tag=tconstruct_upgrade_test]', 'clear @s', 'tp @s 0.5 71 0.5',
                 'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:tinker_station',
                 'give @s tconstruct:small_blade[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
                 'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1',
                 'give @s minecraft:stick 1'):
        cmd(text)
    game.call('interact', '0', '71', '2')
    game.call('wait', '15')
    # The repair button occupies the first cell; dagger is the next source layout.
    game.call('click-gui', '-99', '18')
    game.call('wait', '8')
    click(hotbar('tconstruct:small_blade'))
    click(300)
    click(hotbar('tconstruct:tool_handle'))
    click(301)
    stick = hotbar('minecraft:stick')
    click(stick)
    click(324)
    rejected = game.call('menu')
    assert (rejected['carried'] or {}).get('item') == 'minecraft:stick', rejected
    inputs = {slot['slot']: slot['stack']['item'] for slot in rejected['slots']}
    assert inputs.get(300) == 'tconstruct:small_blade' and inputs.get(301) == 'tconstruct:tool_handle', inputs
    click(stick)
    game.call('click-gui', '85', '10')
    type_text(game, 'mossyy')
    game.call('wait', '15')
    game.call('type-back')
    game.call('wait', '5')
    game.call('screenshot', str(run / 'station-dagger.png'))
    click(324)
    result = game.call('menu')['carried'] or {}
    assert result.get('item') == 'tconstruct:dagger', result
    assert result.get('count') == 2, result
    assert result.get('custom_name') == 'mossy', result
    click(0)
    game.call('close')
    game.stop()
    game = Game(args, run, 'Player', in_frame=args.in_frame, resume=True)
    game.ready()
    inventory = game.call('inventory')
    assert any(stack.get('item') == 'tconstruct:dagger' and 'mossy' in stack.get('components', '') for stack in inventory['hotbar']), inventory
    game.stop()
finally:
    game.cleanup()
print('PASS two-input Station recipe')

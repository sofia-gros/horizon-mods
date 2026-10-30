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
    # Removing an ingredient invalidates the result and clears its old name.
    game.call('click-gui', '85', '10'); type_text(game, 'stale'); game.call('wait', '10')
    click(300); click(300)
    click(324)
    restored = game.call('menu')['carried'] or {}
    assert restored.get('item') == 'tconstruct:dagger' and restored.get('custom_name') is None, restored
    empty = next(s['slot'] for s in game.call('inventory')['hotbar'] if not s.get('item'))
    click(empty)
    for text in ('give @s tconstruct:small_blade[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
                 'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1'):
        cmd(text)
    click(hotbar('tconstruct:small_blade')); click(300)
    click(hotbar('tconstruct:tool_handle')); click(301)
    # Clear an entered name and confirm that the result keeps its default name.
    game.call('click-gui', '85', '10'); type_text(game, 'cancel'); game.call('wait', '10')
    for _ in 'cancel': game.call('type-back')
    game.call('wait', '10'); click(324)
    unnamed = game.call('menu')['carried'] or {}
    assert unnamed.get('item') == 'tconstruct:dagger' and unnamed.get('count') == 2 and unnamed.get('custom_name') is None, unnamed
    empty = next(s['slot'] for s in game.call('inventory')['hotbar'] if not s.get('item'))
    click(empty)
    for text in ('give @s tconstruct:small_blade[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
                 'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1'):
        cmd(text)
    click(hotbar('tconstruct:small_blade')); click(300)
    click(hotbar('tconstruct:tool_handle')); click(301)
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
    named_slot = next(s['slot'] for s in game.call('inventory')['hotbar'] if not s.get('item'))
    click(named_slot)
    # A second identical pair cannot fit on the full cursor stack; inputs
    # must remain untouched when the result is clicked.
    for text in ('give @s tconstruct:small_blade[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
                 'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1'):
        cmd(text)
    click(hotbar('tconstruct:small_blade')); click(300)
    click(hotbar('tconstruct:tool_handle')); click(301)
    game.call('click-gui', '85', '10'); type_text(game, 'mossy'); game.call('wait', '10')
    click(named_slot); click(324)
    rejected = game.call('menu')
    assert (rejected['carried'] or {}).get('count') == 2 and (rejected['carried'] or {}).get('custom_name') == 'mossy', rejected
    inputs = {slot['slot']: slot['stack']['item'] for slot in rejected['slots']}
    assert inputs.get(300) == 'tconstruct:small_blade' and inputs.get(301) == 'tconstruct:tool_handle', inputs
    click(named_slot)
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

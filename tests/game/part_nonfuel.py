"""Verify source ToolPartItem components do not inherit stick furnace fuel."""
import argparse
import tempfile
import time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
p.add_argument('--output-parent', type=Path, default=Path('target'))
a = p.parse_args()
a.output_parent.mkdir(parents=True, exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='part-nonfuel-', dir=a.output_parent.resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
try:
    g.ready()
    for item in ('tconstruct:tool_handle', 'tconstruct:tool_binding', 'tconstruct:fake_ingot'):
        for command in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 minecraft:air',
                        'setblock 0 71 2 minecraft:furnace', 'give @s minecraft:iron_ore 1',
                        f'give @s {item}[minecraft:custom_data={{Material:"tconstruct:wood"}}] 1'):
            g.call('command', command)
        for _ in range(40):
            hotbar = g.call('inventory')['hotbar']
            if hotbar[0].get('item') == 'minecraft:iron_ore' and hotbar[1].get('item') == item:
                break
            time.sleep(.25)
        else:
            raise AssertionError(hotbar)
        g.call('interact', '0', '71', '2')
        g.call('wait', '20')
        for slot in (0, 300, 1, 301):
            g.call('click-slot', str(slot))
            g.call('wait', '5')
        g.call('wait', '100')
        slots = {entry['id']: entry.get('stack') for entry in g.call('ui')['widgets'] if entry['id'].startswith('slot-')}
        assert slots['slot-300']['item'] == 'minecraft:iron_ore', slots
        carried = g.call('inventory')['carried']
        assert slots['slot-301'] is None, slots
        assert carried and carried['item'] == item and carried['count'] == 1, (slots, carried)
        assert slots['slot-302'] is None, slots
        g.call('click-slot', '1')
        g.call('wait', '10')
        g.call('close')
    # An unrelated genuine vanilla fuel still works.
    for command in ('clear @s', 'setblock 0 71 2 minecraft:air',
                    'setblock 0 71 2 minecraft:furnace', 'give @s minecraft:iron_ore 1',
                    'give @s minecraft:stick 1'):
        g.call('command', command)
    for _ in range(40):
        hotbar = g.call('inventory')['hotbar']
        if hotbar[0].get('item') == 'minecraft:iron_ore' and hotbar[1].get('item') == 'minecraft:stick':
            break
        time.sleep(.25)
    else:
        raise AssertionError(hotbar)
    g.call('interact', '0', '71', '2')
    g.call('wait', '20')
    for slot in (0, 300, 1, 301):
        g.call('click-slot', str(slot))
        g.call('wait', '5')
    g.call('wait', '100')
    slots = {entry['id']: entry.get('stack') for entry in g.call('ui')['widgets'] if entry['id'].startswith('slot-')}
    assert slots['slot-301'] is None and g.call('inventory')['carried'] is None, slots
    g.stop()
    print('PASS parts and fake ingot rejected as fuel; vanilla stick is consumed', flush=True)
finally:
    g.cleanup()

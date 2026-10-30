"""Exercise one source-recipe Part Builder path with ordinary GUI interaction."""
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
run = Path(tempfile.mkdtemp(prefix='source-builder-gui-', dir=a.output_parent.resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)

def wait(label, predicate):
    for _ in range(120):
        if predicate():
            print('ok', label, flush=True)
            return
        time.sleep(.2)
    raise AssertionError(label)

def click_slot(slot):
    g.call('click-slot', str(slot))
    g.call('wait', '3')

def hotbar(item):
    return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item') == item)

def pattern_node(label):
    return next((node for node in g.call('ui')['widgets'] if node.get('label') == label and node.get('enabled')), None)

try:
    g.ready()
    for command in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder',
                    'give @s tconstruct:pattern 2', 'give @s minecraft:oak_log 1'):
        g.call('command', command)
    g.call('interact', '0', '71', '2')
    g.call('wait', '10'); g.call('screenshot', str(run / 'empty-builder.png'))
    click_slot(hotbar('tconstruct:pattern')); click_slot(300)
    wait('source pattern grid visible', lambda: pattern_node('Tool Handle') is not None)
    g.call('screenshot', str(run / 'pattern-grid.png'))
    click_slot(hotbar('minecraft:oak_log')); click_slot(301)
    wait('source handle choice visible', lambda: pattern_node('Tool Handle') is not None)
    g.call('click', pattern_node('Tool Handle')['id'])
    g.call('wait', '20')
    g.call('screenshot', str(run / 'source-builder.png'))
    click_slot(322)
    carried = g.call('menu').get('carried') or {}
    assert carried.get('item') == 'tconstruct:tool_handle' and carried.get('count') == 1, carried
    inventory = g.call('inventory')['hotbar']
    empty = next(s['slot'] for s in inventory if not s.get('item'))
    click_slot(empty)
    inventory = g.call('inventory')['hotbar']
    handle = next(s for s in inventory if s.get('item') == 'tconstruct:tool_handle')
    planks = next(s for s in inventory if s.get('item') == 'minecraft:oak_planks')
    assert 'tconstruct:wood' in handle.get('components', ''), handle
    assert planks.get('count') == 3, planks
    g.call('close'); g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    inventory = g.call('inventory')['hotbar']
    assert any(s.get('item') == 'tconstruct:tool_handle' and 'tconstruct:wood' in s.get('components', '') for s in inventory), inventory
    assert any(s.get('item') == 'minecraft:oak_planks' and s.get('count') == 3 for s in inventory), inventory
    g.stop()
    print('PASS source handle, material, leftover and save/reload', flush=True)
finally:
    g.cleanup()

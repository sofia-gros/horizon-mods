"""Exercise source-recipe Part Builder quick-move with ordinary GUI interaction."""
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
p.add_argument('--right', action='store_true')
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

def quick_result():
    g.call('click-slot', '322', *(['right'] if a.right else []), 'shift')

def pattern_node(label):
    return next((node for node in g.call('ui')['widgets'] if node.get('label') == label and node.get('enabled')), None)

try:
    g.ready()
    for command in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder',
                    'give @s tconstruct:pattern 3', 'give @s minecraft:oak_log 2'):
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
    g.call('command', 'setblock 1 71 2 minecraft:chest')
    g.call('wait', '10')
    before_side = g.call('menu')['slots']
    quick_result()
    g.call('wait', '10')
    after_side = g.call('menu')['slots']
    side_inputs = lambda rows: {r['slot']: r['stack'] for r in rows if r.get('slot') in (300,301) and r.get('stack')}
    assert side_inputs(after_side) == side_inputs(before_side), (before_side, after_side)
    g.call('command', 'setblock 1 71 2 minecraft:air')
    g.call('wait', '10')
    g.call('command', 'clear @s')
    g.call('command', 'give @s minecraft:stone 2304')
    g.call('wait', '20')
    before = g.call('menu')['slots']
    assert sum((r.get('stack') or {}).get('count', 0) for r in before if 0 <= r.get('slot', -1) < 36 and (r.get('stack') or {}).get('item') == 'minecraft:stone') == 2304, before
    quick_result()
    g.call('wait', '10')
    after = g.call('menu')['slots']
    inputs = lambda rows: {r['slot']: r['stack'] for r in rows if r.get('slot') in (300,301) and r.get('stack')}
    assert inputs(after) == inputs(before), (before, after)
    g.call('command', 'clear @s')
    g.call('wait', '10')
    quick_result()
    g.call('wait', '20')
    inventory = g.call('inventory')['hotbar']
    print('quick inventory', inventory, 'slots', g.call('menu')['slots'], flush=True)
    handle = next(s for s in inventory if s.get('item') == 'tconstruct:tool_handle')
    planks = next(s for s in inventory if s.get('item') == 'minecraft:oak_planks')
    assert handle.get('count') == 2 and 'tconstruct:wood' in handle.get('components', ''), handle
    assert planks.get('count') == 6, planks
    slots = g.call('menu')['slots']
    assert not any(s.get('slot') == 301 and s.get('stack') for s in slots), slots
    assert any(s.get('slot') == 300 and (s.get('stack') or {}).get('count') == 1 for s in slots), slots
    g.call('close'); g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    inventory = g.call('inventory')['hotbar']
    print('reloaded inventory', inventory, flush=True)
    assert any(s.get('item') == 'tconstruct:tool_handle' and s.get('count') == 2 and 'tconstruct:wood' in s.get('components', '') for s in inventory), inventory
    assert any(s.get('item') == 'minecraft:oak_planks' and s.get('count') == 6 for s in inventory), inventory
    g.stop()
    print('PASS source quick move, repeated crafting, leftover and save/reload', flush=True)
finally:
    g.cleanup()

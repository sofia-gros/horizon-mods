"""Exercise one Part Builder → Station lifecycle in the reusable test world."""
import argparse
import re
import time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
run.mkdir(parents=True, exist_ok=True)
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=(run / 'Player').exists())

def click(slot, *flags):
    g.call('click-slot', str(slot), *flags)
    g.call('wait', '5')

def hotbar(item):
    return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item') == item)

def empty_hotbar():
    return next(s['slot'] for s in g.call('inventory')['hotbar'] if not s.get('item'))

def select(label):
    for _ in range(120):
        node = next((n for n in g.call('ui')['widgets'] if n.get('label') == label and n.get('enabled')), None)
        if node:
            g.call('click', node['id']); g.call('wait', '15'); return
        time.sleep(.2)
    raise AssertionError('missing choice: ' + label)

def slot_values():
    return {v['slot']: v['stack'] for v in g.call('menu')['slots'] if v.get('stack')}

try:
    g.ready()
    for command in ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5',
                    'setblock 0 71 2 minecraft:air', 'setblock 1 71 2 minecraft:air',
                    'setblock 2 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:part_builder',
                    'setblock 1 71 2 minecraft:chest', 'setblock 2 71 2 tconstruct:tinker_station',
                    'give @s tconstruct:pattern 8', 'give @s minecraft:flint 4', 'give @s minecraft:oak_planks 8',
                    'give @s minecraft:stick 1'):
        g.call('command', command)
    g.call('interact', '0', '71', '2'); g.call('wait', '10')
    click(hotbar('minecraft:stick')); click(327)
    assert slot_values()[327]['item'] == 'minecraft:stick'
    click(327); click(empty_hotbar())
    click(hotbar('tconstruct:pattern')); click(300)
    click(hotbar('minecraft:flint')); click(301)
    select('Pick Head'); click(322); click(empty_hotbar())
    click(301); click(empty_hotbar())
    click(hotbar('minecraft:oak_planks')); click(301)
    for label in ('Tool Handle', 'Tool Binding', 'Pick Head'):
        select(label); click(322); click(empty_hotbar())
    g.call('close'); g.call('interact', '2', '71', '2'); g.call('wait', '10')
    for index, item in enumerate(('tconstruct:pick_head', 'tconstruct:tool_handle', 'tconstruct:tool_binding')):
        click(hotbar(item)); click(300 + index)
    click(324)
    assert (g.call('menu')['carried'] or {}).get('item') == 'tconstruct:pickaxe'
    click(empty_hotbar()); g.call('close')
    pick = hotbar('tconstruct:pickaxe'); g.call('hotbar', str(pick + 1))
    g.call('command', 'gamemode survival @s'); g.call('command', 'setblock 0 71 1 minecraft:stone')
    g.call('wait', '10'); g.call('break', '0', '71', '1', '--timeout', '30', timeout=35)
    assert g.call('inventory')['held'].get('damage', 0) > 0
    g.call('command', 'gamemode creative @s'); g.call('interact', '2', '71', '2'); g.call('wait', '10')
    click(pick); click(300); click(hotbar('minecraft:flint')); click(301); click(325)
    assert (g.call('menu')['carried'] or {}).get('item') == 'tconstruct:pickaxe'
    click(pick)
    assert g.call('inventory')['held'].get('damage', 0) == 0
    if 301 in slot_values(): click(301); click(empty_hotbar())
    g.call('close')
    g.call('interact', '2', '71', '2'); g.call('wait', '10')
    click(pick); click(300); click(hotbar('tconstruct:pick_head')); click(301)
    print('swap inputs:', slot_values(), flush=True)
    click(325)
    out = g.call('menu')['carried'] or {}
    print('swap output:', out, flush=True)
    assert out.get('item') == 'tconstruct:pickaxe'
    click(pick)
    assert re.search(r'tic_materials:\s*\[\s*["\']?tconstruct:wood', g.call('inventory')['held'].get('components', ''))
    g.call('screenshot', str(run / 'station-replaced.png'))
    g.call('close'); g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    assert any(s.get('item') == 'tconstruct:pickaxe' and re.search(r'tic_materials:\s*\[\s*["\']?tconstruct:wood', s.get('components', '')) for s in g.call('inventory')['hotbar'])
    g.stop(); print('PASS builder, side, build, use, repair, swap and reload', flush=True)
finally:
    g.cleanup()

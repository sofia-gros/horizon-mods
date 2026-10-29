"""Exercise native 26.3 crafting of both source blank-cast variants."""
import argparse
from pathlib import Path
import tempfile
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='blank-cast-crafting-', dir=Path('target').resolve()))
g = Game(a, run, 'Player', in_frame=a.in_frame)
def click(slot, *flags):
    g.call('click-slot', str(slot), *flags); g.call('wait', '3')
try:
    g.ready()
    for cmd in ('fill -2 70 -2 2 70 3 minecraft:stone', 'tp @s 0.5 71 0.5', 'setblock 0 71 3 minecraft:crafting_table', 'clear @s', 'give @s minecraft:sand 2', 'give @s minecraft:red_sand 2', 'gamemode survival @s'):
        g.call('command', cmd)
    for item, result in [('minecraft:sand', 'tconstruct:blank_sand_cast'), ('minecraft:red_sand', 'tconstruct:blank_red_sand_cast')]:
        g.call('interact', '0', '71', '3'); g.call('wait', '5')
        slot = next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item') == item)
        click(slot); click(100, 'right'); click(slot)
        slots = {s['slot']: s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
        assert slots.get(200, {}).get('item') == result and slots[200]['count'] == 4, slots
        click(200, 'shift'); g.call('close')
        assert g.call('inventory')['counts'].get(result) == 4
        assert g.call('inventory')['counts'].get(item) == 1
    g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    counts = g.call('inventory')['counts']
    assert counts.get('tconstruct:blank_sand_cast') == 4 and counts.get('tconstruct:blank_red_sand_cast') == 4, counts
    print('PASS both 26.3 blank cast recipes and save/reload', flush=True)
    g.stop()
finally:
    g.cleanup()

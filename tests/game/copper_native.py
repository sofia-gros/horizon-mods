"""Check the 26.3 native copper nugget path and the adapted source pack."""
import argparse
import json
import tempfile
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
source = a.resources / 'tconstruct-original-pack/data'
tag = json.loads((source / 'forge/tags/items/nuggets/copper.json').read_text())
assert 'minecraft:copper_nugget' in tag['values'] and 'tconstruct:copper_nugget' not in tag['values']
recipe_dir = source / 'tconstruct/recipes/common/materials'
for name in ('copper_ingot_from_nuggets.json', 'copper_nugget_from_ingot.json'):
    text = (recipe_dir / name).read_text()
    assert 'minecraft:copper_nugget' in text and 'tconstruct:copper_nugget' not in text
run = Path(tempfile.mkdtemp(prefix='native-copper-', dir=Path('target').resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
def click(slot, *flags):
    g.call('click-slot', str(slot), *flags); g.call('wait', '3')
try:
    g.ready()
    for cmd in ('fill -2 70 -2 2 70 3 minecraft:stone', 'tp @s 0.5 71 0.5',
                'setblock 0 71 3 minecraft:crafting_table', 'clear @s',
                'give @s minecraft:copper_ingot 1', 'gamemode survival @s'):
        g.call('command', cmd)
    g.call('interact', '0', '71', '3'); g.call('wait', '5')
    slot = next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item') == 'minecraft:copper_ingot')
    click(slot); click(100)
    result = next(s['stack'] for s in g.call('menu')['slots'] if s.get('slot') == 200 and s.get('stack'))
    assert result['item'] == 'minecraft:copper_nugget' and result['count'] == 9, result
    g.call('screenshot', str(run / 'copper-nugget.png'))
    click(200, 'shift'); g.call('close'); g.call('save')
    assert g.call('inventory')['counts'].get('minecraft:copper_nugget') == 9
    g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    assert g.call('inventory')['counts'].get('minecraft:copper_nugget') == 9
    g.stop(); print('PASS native copper nugget, adapted source references and save/reload', flush=True)
finally:
    g.cleanup()

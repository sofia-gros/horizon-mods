"""Check a table's source recipe lifetime across a datapack reload."""
import argparse, json, tempfile, time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='source-builder-reload-', dir=Path('target').resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
def click(slot):
    g.call('click-slot', str(slot)); g.call('wait', '3')
def node():
    return next((n for n in g.call('ui')['widgets'] if n.get('label') == 'Pick Head' and n.get('enabled')), None)
try:
    g.ready()
    for command in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder',
                    'give @s tconstruct:pattern 2', 'give @s minecraft:flint 3'):
        g.call('command', command)
    g.call('interact', '0', '71', '2')
    click(0); click(300); click(1); click(301)
    for _ in range(120):
        if node(): break
        time.sleep(.2)
    assert node(), g.call('ui')
    g.call('click', node()['id']); g.call('wait', '20')
    pack = g.root / 'saves/Probe/datapacks/source-lifetime'
    path = pack / 'data/tconstruct/recipes/tools/parts/builder/pick_head.json'
    path.parent.mkdir(parents=True)
    (pack / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"source lifetime"}}')
    recipe = json.loads((Path(__file__).resolve().parents[2] / 'content/data/tconstruct/recipes/tools/parts/builder/pick_head.json').read_text())
    assert recipe['cost'] == 2, recipe
    recipe['cost'] = 3
    path.write_text(json.dumps(recipe))
    g.call('command', 'reload'); g.call('wait', '60')
    click(322)
    assert (g.call('menu').get('carried') or {}).get('item') == 'tconstruct:pick_head'
    slots = {s['slot']: s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
    assert slots.get(301, {}).get('count') == 1, slots
    print('PASS cached original cost 2 after reload changed cost to 3', flush=True)
    g.stop()
finally:
    g.cleanup()

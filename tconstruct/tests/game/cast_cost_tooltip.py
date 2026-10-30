"""Capture source cast cost before and after a world recipe reload."""
import argparse, json, tempfile
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='cast-cost-', dir=Path('target').resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
try:
    g.ready()
    for cmd in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder', 'give @s tconstruct:pick_head_cast 1'):
        g.call('command', cmd)
    g.call('wait', '30'); g.call('interact', '0', '71', '2'); g.call('wait', '60')
    node = next(n for n in g.call('ui')['widgets'] if n['id'] == 'slot-0')
    x, y, _, _ = node['bounds']
    def capture(name):
        g.call('scroll-at', str(x + 8), str(y + 8), '0'); g.call('wait', '30'); g.call('screenshot', str(run / name))
    capture('before.png')
    pack = g.root / 'saves/Probe/datapacks/cast-cost'
    (pack / 'data/tconstruct/recipes/tools/parts/casting').mkdir(parents=True)
    (pack / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"cast cost"}}')
    for kind in ('gold', 'sand'):
        name = f'pick_head_{kind}_cast.json'
        src = a.resources / 'tconstruct-original-pack/data/tconstruct/recipes/tools/parts/casting' / name
        recipe = json.loads(src.read_text()); recipe['item_cost'] = 3
        (pack / 'data/tconstruct/recipes/tools/parts/casting' / name).write_text(json.dumps(recipe))
    g.call('command', 'reload'); g.call('wait', '100'); capture('after.png')
    print('Captured both tooltips; inspect screenshots for source cost 2 -> 3', flush=True)
    g.stop()
finally:
    g.cleanup()

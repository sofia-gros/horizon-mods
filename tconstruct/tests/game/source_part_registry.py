"""Exercise source part IDs in the actual item registry, not crafting recipes."""
import argparse
import json
import tempfile
import time
from pathlib import Path
from harness import Game
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools'))
from import_part_visuals import PARTS

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
p.add_argument('--count', type=int, default=len(PARTS), choices=range(1, len(PARTS)+1))
p.add_argument('--output-parent', type=Path, default=Path('target'))
a = p.parse_args()
a.output_parent.mkdir(parents=True, exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='source-part-registry-', dir=a.output_parent.resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
expected = {'tconstruct:' + part for part in PARTS[:a.count]}
def check():
    inventory = g.call('inventory')
    stacks = [s for s in inventory['hotbar'] + inventory['main'] if s.get('item') in expected]
    if len(stacks) != len(expected):
        return False
    assert {s['item'] for s in stacks} == expected
    assert all('tconstruct:wood' in s.get('components', '') for s in stacks), stacks
    return True
try:
    g.ready()
    for cmd in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder'):
        g.call('command', cmd)
    for part in PARTS[:a.count]:
        g.call('command', f'give @s tconstruct:{part}[minecraft:custom_data={{Material:"tconstruct:wood"}}] 1')
    for _ in range(80):
        if check():
            break
        time.sleep(.25)
    else:
        raise AssertionError(g.call('inventory'))
    g.call('interact', '0', '71', '2')
    g.call('wait', '30')
    ui = g.call('ui')
    shown = [w['stack']['item'] for w in ui['widgets'] if w.get('stack') and w['stack'].get('item') in expected]
    assert set(shown) == expected, shown
    (run / 'ui.json').write_text(json.dumps(ui))
    g.call('screenshot', str(run / 'source-parts.png'))
    g.call('wait', '60')
    g.call('screenshot', str(run / 'source-parts-later.png'))
    g.call('close')
    g.call('save')
    g.stop()
    g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)
    g.ready()
    for _ in range(80):
        if check():
            break
        time.sleep(.25)
    else:
        raise AssertionError(g.call('inventory'))
    g.stop()
    print(f'PASS {a.count} source part IDs and stored material after reload; inspect pixels separately', flush=True)
finally:
    g.cleanup()

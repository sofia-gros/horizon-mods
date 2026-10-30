"""Inspect the registered source fake ingot; this does not test crafting."""
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
run = Path(tempfile.mkdtemp(prefix='source-fake-ingot-', dir=a.output_parent.resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
materials = ('tconstruct:bronze', 'tconstruct:silver', 'tconstruct:unknown')
def check():
    stacks = [s for s in g.call('inventory')['hotbar'] if s.get('item') == 'tconstruct:fake_ingot']
    return len(stacks) == len(materials) and all(any(m in s.get('components', '') for s in stacks) for m in materials)
try:
    g.ready()
    for cmd in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder'):
        g.call('command', cmd)
    for material in materials:
        g.call('command', f'give @s tconstruct:fake_ingot[minecraft:custom_data={{Material:"{material}"}}] 1')
    for _ in range(40):
        if check(): break
        time.sleep(.25)
    else: raise AssertionError(g.call('inventory'))
    g.call('interact', '0', '71', '2')
    g.call('wait', '30')
    g.call('screenshot', str(run / 'fake-ingots.png'))
    for slot, name in ((0, 'bronze'), (1, 'silver'), (2, 'unknown')):
        node = next(w for w in g.call('ui')['widgets'] if w['id'] == f'slot-{slot}')
        x, y, _, _ = node['bounds']
        g.call('scroll-at', str(x + 8), str(y + 8), '0')
        g.call('wait', '10')
        g.call('screenshot', str(run / f'{name}.png'))
    g.call('close')
    g.call('save')
    g.stop()
    g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)
    g.ready()
    for _ in range(40):
        if check(): break
        time.sleep(.25)
    else: raise AssertionError(g.call('inventory'))
    g.stop()
    print('PASS fake ingot command data and save/reload; inspect pixels and names separately', flush=True)
finally:
    g.cleanup()

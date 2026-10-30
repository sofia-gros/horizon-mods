"""Capture the source dummy plating tooltip in the shared inventory path."""
import argparse, tempfile, time
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='dummy-tooltip-', dir=Path('target').resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
try:
    g.ready()
    for cmd in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder', 'give @s tconstruct:helmet_plating_dummy 1'):
        g.call('command', cmd)
    for _ in range(40):
        if any(s.get('item') == 'tconstruct:helmet_plating_dummy' for s in g.call('inventory')['hotbar']): break
        time.sleep(.25)
    else: raise AssertionError('dummy item not received')
    g.call('interact', '0', '71', '2'); g.call('wait', '30')
    node = next(n for n in g.call('ui')['widgets'] if n['id'] == 'slot-0')
    x, y, _, _ = node['bounds']
    g.call('scroll-at', str(x + 8), str(y + 8), '0'); g.call('wait', '10')
    g.call('screenshot', str(run / 'dummy-tooltip.png'))
    print('Captured tooltip; inspect screenshot for translated source text', flush=True)
    g.stop()
finally:
    g.cleanup()

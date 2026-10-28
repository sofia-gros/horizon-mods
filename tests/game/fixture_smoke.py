"""Start vanilla Horizon with the independent Mixins fixture and click a table."""
import argparse
import tempfile
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
run = Path(tempfile.mkdtemp(prefix='fixture-smoke-', dir=a.output_parent.resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
try:
    g.ready()
    g.call('command', 'tp @s 0.5 71 0.5')
    g.call('command', 'setblock 0 71 2 minecraft:crafting_table')
    g.call('wait', '10')
    g.call('interact', '0', '71', '2')
    g.call('click-slot', '100')
    g.call('wait', '10')
    g.stop()
    assert 'independent fixture station-click hook executed' in (run / 'Player/game.log').read_text()
    print('PASS independent fixture hook', flush=True)
finally:
    g.cleanup()

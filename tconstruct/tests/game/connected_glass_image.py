"""Capture a clear connected-glass example in the reusable test world."""
import argparse
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
for name in ('game', 'ctl', 'resources'):
    p.add_argument('--' + name, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)
try:
    g.ready()
    for text in ('gamemode creative @s', 'time set day', 'clear @s',
                 'fill -3 70 -3 4 70 6 minecraft:stone', 'fill -3 71 -3 4 75 6 minecraft:air',
                 'setblock 0 71 2 tconstruct:clear_glass', 'setblock 1 71 2 tconstruct:clear_glass',
                 'setblock 2 71 2 tconstruct:red_clear_stained_glass',
                 'setblock 3 71 2 tconstruct:seared_glass', 'gamemode spectator @s', 'tp @s 0.5 72 -1.5'):
        g.call('command', text); g.call('wait', '8')
    g.call('look', '-8', '25'); g.call('wait', '240')
    g.call('screenshot', str(run / 'connected-glass-guide.png'))
    g.call('command', 'gamemode creative @s'); g.call('wait', '8'); g.stop()
    print('PASS connected glass example captured; inspect image', flush=True)
finally:
    g.cleanup()

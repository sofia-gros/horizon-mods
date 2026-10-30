"""Capture vertical pane interfaces and a vanilla glass regression in the test world."""
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


def command(text):
    g.call('command', text)
    g.call('wait', '8')


def capture(name, position, yaw, pitch):
    command('tp @s ' + position)
    g.call('look', str(yaw), str(pitch)); g.call('wait', '240')
    g.call('screenshot', str(run / name))


try:
    g.ready()
    for text in ('gamemode creative @s', 'time set day', 'fill 13 70 -2 18 70 4 minecraft:stone',
                 'fill 13 71 -2 18 76 4 minecraft:air', 'fill 13 69 -2 18 69 1 minecraft:stone',
                 'fill 13 70 -2 18 70 1 minecraft:air',
                 'setblock 15 71 2 tconstruct:clear_glass_pane',
                 'setblock 16 71 2 tconstruct:clear_glass_pane',
                 'setblock 15 72 2 tconstruct:clear_glass_pane'):
        command(text)
    assert 'east=true' in g.call('block', '15', '71', '2')['state']
    assert 'east=false' in g.call('block', '15', '72', '2')['state']
    command('gamemode spectator @s')
    capture('pane-unmatched-top.png', '14.5 74.5 0.5', -26.6, 58)
    capture('pane-unmatched-bottom.png', '14.5 69.1 0.5', -26.6, -30)
    command('gamemode creative @s')
    command('setblock 16 72 2 tconstruct:clear_glass_pane')
    assert 'east=true' in g.call('block', '15', '72', '2')['state']
    command('gamemode spectator @s')
    capture('pane-matching-top.png', '14.5 74.5 0.5', -26.6, 58)
    command('gamemode creative @s')
    for text in ('setblock 15 71 2 minecraft:glass', 'setblock 16 71 2 minecraft:glass',
                 'setblock 15 72 2 minecraft:air', 'setblock 16 72 2 minecraft:air'):
        command(text)
    command('gamemode spectator @s')
    capture('pane-vanilla-glass.png', '14.5 74.5 0.5', -26.6, 58)
    command('gamemode creative @s')
    g.stop()
    print('PASS pane states and vanilla glass; inspect the four screenshots', flush=True)
finally:
    g.cleanup()

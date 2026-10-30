"""Capture source-material part sprites from ordinary game GUI rendering.

The variant items are supplied by diagnostic commands; this test does not
claim that the current Part Builder can craft every displayed material.
"""
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
run = Path(tempfile.mkdtemp(prefix='source-part-visuals-', dir=a.output_parent.resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
try:
    g.ready()
    for cmd in (
        'clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder',
        'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
        'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:wood"}] 1',
        'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:copper#oxidized"}] 1',
    ):
        g.call('command', cmd)
    expected_materials = ('tconstruct:flint', 'tconstruct:wood', 'tconstruct:copper#oxidized')
    # Standard client/server runs deliver inventory updates asynchronously.
    for _ in range(40):
        hotbar = g.call('inventory')['hotbar']
        materials = [s.get('components', '') for s in hotbar if s.get('item') == 'tconstruct:pick_head']
        if all(any(expected in data for data in materials) for expected in expected_materials):
            break
        time.sleep(0.25)
    else:
        raise AssertionError((expected_materials, materials))
    time.sleep(1)
    g.call('interact', '0', '71', '2')
    # Allow the standard client to receive its complete opened-menu snapshot.
    g.call('wait', '30')
    g.call('screenshot', str(run / 'source-part-visuals.png'))
    # Swap two different materials through the same GUI slots. This exercises
    # repeated rendering at identical coordinates rather than only first draw.
    for slot in ('0', '2', '0'):
        g.call('click-slot', slot)
        g.call('wait', '3')
    swapped = g.call('inventory')['hotbar']
    assert 'tconstruct:copper#oxidized' in swapped[0].get('components', ''), swapped
    assert 'tconstruct:flint' in swapped[2].get('components', ''), swapped
    g.call('scroll-at', '0', '0', '0')
    g.call('wait', '5')
    g.call('screenshot', str(run / 'source-part-visuals-swapped.png'))
    g.call('close')
    g.call('save')
    g.stop()
    print('PASS source-material stacks and GUI screenshot; verify pixels separately', flush=True)
finally:
    g.cleanup()

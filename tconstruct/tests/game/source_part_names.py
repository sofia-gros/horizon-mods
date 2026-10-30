"""Capture source-material part names and explicit-name preservation in game."""
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
p.add_argument('--language', default='en_us')
p.add_argument('--output-parent', type=Path, default=Path('target'))
a = p.parse_args()
a.output_parent.mkdir(parents=True, exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='source-part-names-', dir=a.output_parent.resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
try:
    g.ready()
    if a.language != 'en_us':
        g.call('options-set', 'lang', a.language)
    for cmd in (
        'clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder',
        'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
        'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:wood"}] 1',
        'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:copper#oxidized"}] 1',
        '''give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:wood"},minecraft:custom_name='"Named Head"'] 1''',
        '''give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:wood"},minecraft:item_name='"Stored Head"'] 1''',
    ):
        g.call('command', cmd)
    for _ in range(40):
        hotbar = g.call('inventory')['hotbar']
        if len([s for s in hotbar if s.get('item') == 'tconstruct:pick_head']) == 5:
            break
        time.sleep(.25)
    else:
        raise AssertionError(hotbar)
    assert 'Named Head' in hotbar[3].get('components', '')
    assert 'Stored Head' in hotbar[4].get('components', '')
    g.call('interact', '0', '71', '2')
    g.call('wait', '30')
    for slot, label in enumerate(('flint', 'wood', 'oxidized', 'custom-name', 'item-name')):
        node = next(n for n in g.call('ui')['widgets'] if n['id'] == f'slot-{slot}')
        x, y, _, _ = node['bounds']
        g.call('scroll-at', str(x + 8), str(y + 8), '0')
        g.call('wait', '10')
        g.call('screenshot', str(run / f'{label}.png'))
    g.call('close')
    g.call('save')
    g.stop()
    g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)
    g.ready()
    hotbar = g.call('inventory')['hotbar']
    assert 'Named Head' in hotbar[3].get('components', '')
    assert 'Stored Head' in hotbar[4].get('components', '')
    assert 'tconstruct:wood' in hotbar[1].get('components', '')
    g.stop()
    print('PASS name components, save/reload and tooltip screenshots; inspect text separately', flush=True)
finally:
    g.cleanup()

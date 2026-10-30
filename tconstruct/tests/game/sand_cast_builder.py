"""Make a source sand cast from a command-supplied blank cast."""
import argparse, tempfile, time
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--red', action='store_true')
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='sand-cast-builder-', dir=Path('target').resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
def click(slot):
    g.call('click-slot', str(slot)); g.call('wait', '3')
kind = 'red_sand' if a.red else 'sand'
try:
    g.ready()
    for cmd in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder', f'give @s tconstruct:blank_{kind}_cast 1'):
        g.call('command', cmd)
    g.call('interact', '0', '71', '2'); click(0); click(300)
    for _ in range(120):
        node = next((n for n in g.call('ui')['widgets'] if n.get('label') == 'Adze Head' and n.get('enabled')), None)
        if node: break
        time.sleep(.2)
    assert node, g.call('ui')
    g.call('click', node['id']); g.call('wait', '20'); g.call('screenshot', str(run/'preview.png'))
    click(322)
    result = g.call('menu').get('carried') or {}
    assert result.get('item') == f'tconstruct:adze_head_{kind}_cast' and result.get('count') == 1, result
    slots = {r['slot']: r['stack'] for r in g.call('menu')['slots'] if r.get('stack')}
    assert 300 not in slots, slots
    click(1); g.call('close'); g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    assert any(s.get('item') == f'tconstruct:adze_head_{kind}_cast' for s in g.call('inventory')['hotbar'])
    print('PASS source sand cast output, input consumption and save/reload', flush=True)
    g.stop()
finally:
    g.cleanup()

"""Check vanilla item custom_data through multiplayer menu and world reload."""
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
a = p.parse_args()
root = Path(tempfile.mkdtemp(prefix='network-custom-data-', dir=Path('target').resolve()))
print('run', root, flush=True)
g = None

def wait(label, predicate):
    for _ in range(100):
        if predicate():
            print('ok', label, flush=True)
            return
        time.sleep(.2)
    raise AssertionError(label)

def slot(index):
    return next((s for s in g.call('screen').get('slots', []) if s['slot'] == index and s.get('item')), None)

def has_proof(stack):
    return bool(stack and stack.get('item') == 'minecraft:diamond'
                and 'proof:17' in str(stack.get('components', '')))

try:
    g = Game(a, root, 'Host', in_frame=a.in_frame); g.ready()
    for command in ('fill -2 70 -1 3 70 4 minecraft:stone', 'tp @s -0.5 71 0.5',
                    'setblock 0 71 2 minecraft:chest', 'clear @s',
                    'give @s minecraft:diamond[minecraft:custom_data={proof:17}] 1'):
        g.call('command', command)
    wait('client inventory', lambda: any(has_proof(s) for s in g.call('inventory')['hotbar']))
    wait('client chest', lambda: g.call('block', '0', '71', '2').get('block') == 'minecraft:chest')
    g.call('interact', '0', '71', '2')
    wait('container open', lambda: any(w['id'] == 'slot-300' for w in g.call('ui')['widgets']))
    hotbar = next(s['slot'] for s in g.call('inventory')['hotbar'] if has_proof(s))
    g.call('click-slot', str(hotbar)); g.call('wait', '3'); g.call('click-slot', '300')
    wait('diamond stored', lambda: has_proof(slot(300)))
    g.call('close'); g.stop(); g.cleanup(); g = None
    g = Game(a, root, 'Host', in_frame=a.in_frame, resume=True); g.ready()
    g.call('interact', '0', '71', '2')
    wait('saved diamond', lambda: has_proof(slot(300)))
    g.call('click-slot', '300'); g.call('wait', '3'); g.call('click-slot', str(hotbar)); g.call('wait', '3'); g.call('close')
    wait('retrieved diamond', lambda: any(has_proof(s) for s in g.call('inventory')['hotbar']))
    g.stop(); g.cleanup(); g = None
    g = Game(a, root, 'Host', in_frame=a.in_frame, resume=True); g.ready()
    wait('saved inventory', lambda: any(has_proof(s) for s in g.call('inventory')['hotbar']))
    g.stop(); g.cleanup(); g = None
finally:
    if g is not None:
        g.cleanup()

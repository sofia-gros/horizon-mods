"""Exercise an adjacent inventory and source crafting through the builder GUI."""
import argparse
import json
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
run = Path(tempfile.mkdtemp(prefix='builder-side-', dir=Path('target').resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
def click(slot, *flags):
    g.call('click-slot', str(slot), *flags); g.call('wait', '3')
def slots():
    return {row['slot']: row['stack'] for row in g.call('menu')['slots'] if row.get('stack')}
def wait(label, predicate):
    for _ in range(100):
        if predicate(): return
        time.sleep(.2)
    raise AssertionError((label, g.call('menu')))
try:
    g.ready()
    for command in ('fill -2 70 -2 3 70 3 minecraft:stone', 'clear @s', 'tp @s 0.5 71 0.5',
                    'setblock 0 71 2 tconstruct:part_builder', 'setblock 1 71 2 minecraft:chest',
                    'give @s tconstruct:pattern 2', 'give @s minecraft:oak_log 1', 'give @s minecraft:stick 1'):
        g.call('command', command)
    g.call('interact', '0', '71', '2'); g.call('wait', '10')
    click(2); click(327)
    wait('side input', lambda: slots().get(327, {}).get('item') == 'minecraft:stick')
    g.call('screenshot', str(run / 'builder-side.png'))
    click(327); click(2)
    click(0); click(300); click(1); click(301)
    wait('pattern', lambda: any(n.get('label') == 'Tool Handle' and n.get('enabled') for n in g.call('ui')['widgets']))
    node = next(n for n in g.call('ui')['widgets'] if n.get('label') == 'Tool Handle' and n.get('enabled'))
    g.call('click', node['id']); g.call('wait', '15')
    g.call('command', 'clear @s'); g.call('command', 'give @s minecraft:stone 2304'); g.call('wait', '15')
    click(322, 'shift'); g.call('wait', '15')
    wait('side output', lambda: slots().get(327, {}).get('item') == 'tconstruct:tool_handle')
    assert slots()[327]['count'] == 1 and 301 not in slots(), slots()
    g.call('close'); g.call('save'); g.stop(); g.cleanup()
    stats = list((g.root / 'saves/Probe/players/stats').glob('*.json'))
    assert stats, 'missing saved player stats'
    data = json.loads(stats[0].read_text())
    assert data.get('stats', {}).get('minecraft:crafted', {}).get('tconstruct:tool_handle', 0) >= 1, data
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    g.call('interact', '1', '71', '2'); g.call('wait', '10')
    assert any(row.get('stack', {}).get('item') == 'tconstruct:tool_handle' for row in g.call('menu')['slots']), g.call('menu')
    g.stop(); print('PASS side click, result transfer, crafted stat and save/reload', flush=True)
finally:
    g.cleanup()

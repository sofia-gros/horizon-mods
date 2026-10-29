"""Craft a source wood part from vanilla planks after registering source planks."""
import argparse, tempfile, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a=p.parse_args()
run=Path(tempfile.mkdtemp(prefix='oak-plank-builder-', dir=Path('target').resolve()))
print('run', run, flush=True)
g=Game(a,run,'Player',in_frame=a.in_frame)
def click(slot): g.call('click-slot', str(slot)); g.call('wait','3')
def node(): return next((n for n in g.call('ui')['widgets'] if n.get('label')=='Tool Handle' and n.get('enabled')),None)
try:
    g.ready()
    for cmd in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','give @s tconstruct:pattern 2','give @s minecraft:oak_planks 2'):
        g.call('command',cmd)
    g.call('interact','0','71','2'); click(0);click(300);click(1);click(301)
    for _ in range(120):
        if node(): break
        time.sleep(.2)
    assert node(), g.call('menu')
    g.call('click',node()['id']);g.call('wait','20');g.call('screenshot',str(run/'oak-plank-preview.png'))
    click(322)
    carried=g.call('menu').get('carried') or {}
    assert carried.get('item')=='tconstruct:tool_handle' and carried.get('count')==1,carried
    slots={s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
    assert slots.get(301,{}).get('item')=='minecraft:oak_planks' and slots[301]['count']==1,slots
    click(2);g.call('close');g.call('save');g.stop();g.cleanup()
    g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
    assert any(s.get('item')=='tconstruct:tool_handle' and 'tconstruct:wood' in s.get('components','') for s in g.call('inventory')['hotbar'])
    print('PASS oak plank source material, consumption and save/reload',flush=True);g.stop()
finally:
    g.cleanup()

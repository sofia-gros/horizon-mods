"""Check source recycling selection; output transaction is not yet wired."""
import argparse, tempfile, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
p.add_argument('--game',type=Path,required=True)
p.add_argument('--ctl',type=Path,required=True)
p.add_argument('--resources',type=Path,required=True)
p.add_argument('--in-frame',action='store_true')
a=p.parse_args()
run=Path(tempfile.mkdtemp(prefix='recycling-preview-',dir=Path('target').resolve()))
print('run',run,flush=True)
g=Game(a,run,'Player',in_frame=a.in_frame)
def click(slot):
    g.call('click-slot',str(slot));g.call('wait','3')
try:
    g.ready()
    for c in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder',
              'give @s tconstruct:venom_bottle 1','give @s minecraft:stone_axe 1'):
        g.call('command',c)
    g.call('interact','0','71','2')
    click(0);click(300);click(1);click(301)
    for _ in range(120):
        node=next((n for n in g.call('ui')['widgets'] if n.get('label')=='Block' and n.get('enabled')),None)
        if node: break
        time.sleep(.2)
    assert node, g.call('ui')
    g.call('click',node['id']);g.call('wait','20')
    g.call('screenshot',str(run/'recycling-choice.png'))
    print('PASS source stone axe recycling choice; result not yet connected',flush=True)
    g.stop()
finally:
    g.cleanup()

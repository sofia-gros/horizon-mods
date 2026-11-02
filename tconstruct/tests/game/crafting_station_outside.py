import argparse,sys
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to(Path(__file__).resolve().parents[2] / 'target');g=Game(a,run,'Player',resume=True,in_frame=True)
def cmd(s):g.call('command',s);g.call('wait','5')
def cells():return {int(w['id'][5:]):w['stack'] for w in g.call('ui')['widgets'] if w['id'].startswith('slot-') and w.get('stack')}
try:
 g.ready()
 for s in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:crafting_station','give @s minecraft:oak_planks 4'):cmd(s)
 g.call('select','9');g.call('interact','0','71','2');g.call('wait','10');g.call('click-slot','0');g.call('wait','8');g.call('click-slot','100');g.call('wait','8');assert cells().get(100,{}).get('count')==4,cells();g.call('click-slot','100');g.call('wait','8');g.call('click-at','5','5');g.call('wait','20');entities=g.call('entities','8','item')['entities'];dropped=sum(e.get('item',{}).get('count',0) for e in entities if e.get('item',{}).get('item')=='minecraft:oak_planks');assert 100 not in cells() and dropped==4,(cells(),entities);g.call('close');g.call('wait','3');assert not g.call('inventory')['counts'].get('minecraft:oak_planks');g.call('save');g.stop();print('PASS outside drop kept exactly four items',flush=True)
finally:g.cleanup()

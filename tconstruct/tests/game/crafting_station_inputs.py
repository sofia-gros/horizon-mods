import argparse,sys
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to(Path(__file__).resolve().parents[2] / 'target');g=Game(a,run,'Player',resume=True,in_frame=True)
def cmd(s):g.call('command',s);g.call('wait','5')
def click(n):g.call('click-slot',str(n));g.call('wait','8')
def cells():return {int(w['id'][5:]):w['stack'] for w in g.call('ui')['widgets'] if w['id'].startswith('slot-') and w.get('stack')}
def center(slot):
 w=next(w for w in g.call('ui')['widgets'] if w['id']==f'slot-{slot}');x,y,wid,hei=w['bounds'];return (x+wid/2,y+hei/2)
try:
 g.ready()
 for s in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:crafting_station','give @s minecraft:oak_planks 8'):cmd(s)
 g.call('select','9');g.call('interact','0','71','2');g.call('wait','10');click(0);p0=center(100);p1=center(101);g.call('drag-at',*map(str,p0+p1));g.call('wait','15');c=cells();print('DRAG',c,flush=True);assert sum(c.get(i,{}).get('count',0) for i in (100,101))==8,c
 g.call('scroll-at',*map(str,p0),'0');g.call('key','2','tap');g.call('wait','10');c=cells();print('SWAP',c,flush=True);assert 100 not in c and c.get(1,{}).get('count')==4,c
 g.call('close');g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',resume=True,in_frame=True);g.ready();g.call('select','9');g.call('interact','0','71','2');g.call('wait','10');c=cells();print('RELOAD',c,flush=True);assert 100 not in c and c.get(101,{}).get('count')==4 and c.get(1,{}).get('count')==4,c;g.call('close');g.stop();print('PASS drag and numeric swap survive reload',flush=True)
finally:g.cleanup()

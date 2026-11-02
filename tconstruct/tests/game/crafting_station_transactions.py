import argparse,sys
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to(Path(__file__).resolve().parents[2] / 'target');g=Game(a,run,'Player',resume=True,in_frame=True)
def cmd(s):g.call('command',s);g.call('wait','5')
def click(i,*f):g.call('click-slot',str(i),*f);g.call('wait','8')
def cells():return {int(w['id'][5:]):w['stack'] for w in g.call('ui')['widgets'] if w['id'].startswith('slot-') and w.get('stack')}
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def opench():g.call('select','9');g.call('interact','0','71','2');g.call('wait','10')
def put(item,targets):
 for target in targets:
  slot=hot(item);click(slot);click(100+target,'right');click(slot)
try:
 g.ready()
 for s in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:crafting_station','give @s minecraft:oak_planks 2'):cmd(s)
 opench();click(hot('minecraft:oak_planks'));click(100);g.call('wait','20');c=cells();assert c.get(100,{}).get('count')==2,c;g.call('close');g.call('save');g.stop();g.cleanup()
 g=Game(a,run,'Player',resume=True,in_frame=True);g.ready();opench();c=cells();assert c.get(100,{}).get('count')==2,c
 click(100);click(100,'right');click(103,'right');c=cells();assert c.get(200,{}).get('item')=='minecraft:stick',c
 click(200);click(0);g.call('close');inv=g.call('inventory');assert inv['counts'].get('minecraft:stick')==4 and not inv['counts'].get('minecraft:oak_planks'),inv
 cmd('give @s minecraft:oak_planks 2');opench();put('minecraft:oak_planks',[0,3]);assert cells().get(200,{}).get('item')=='minecraft:stick',cells();click(200,'shift');g.call('close');inv=g.call('inventory');assert inv['counts'].get('minecraft:stick')==8 and not inv['counts'].get('minecraft:oak_planks'),inv
 for item,n in [('minecraft:milk_bucket',3),('minecraft:sugar',2),('minecraft:egg',1),('minecraft:wheat',3)]:cmd('give @s '+item+' '+str(n))
 opench();put('minecraft:milk_bucket',[0,1,2]);put('minecraft:sugar',[3,5]);put('minecraft:egg',[4]);put('minecraft:wheat',[6,7,8]);assert cells().get(200,{}).get('item')=='minecraft:cake',cells();click(200);click(1);c=cells();assert all(c.get(i,{}).get('item')=='minecraft:bucket' for i in (100,101,102)),c;g.call('close');inv=g.call('inventory');assert inv['counts'].get('minecraft:cake')==1,inv
 g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',resume=True,in_frame=True);g.ready();opench();c=cells();assert all(c.get(i,{}).get('item')=='minecraft:bucket' for i in (100,101,102)) and all(i not in c for i in range(103,109)),c;g.call('close');inv=g.call('inventory');assert inv['counts'].get('minecraft:stick')==8 and inv['counts'].get('minecraft:cake')==1,inv;g.stop();print('PASS local grid persistence, normal/shift craft, three container remainders and reload',flush=True)
finally:g.cleanup()

import argparse,sys,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to(Path(__file__).resolve().parents[2] / 'target');g=Game(a,run,'Player',resume=True,in_frame=True)
def cmd(s):g.call('command',s);g.call('wait','6')
def click(n):g.call('click-slot',str(n));g.call('wait','10')
def cells():return {int(w['id'][5:]):w['stack'] for w in g.call('ui')['widgets'] if w['id'].startswith('slot-') and w.get('stack')}
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def opench():g.call('select','9');g.call('interact','0','71','2');g.call('wait','12')
def give(item,n):cmd('give @s '+item+' '+str(n))
try:
 g.ready();cmd('gamemode creative @s');cmd('tp @s 0.5 71 0.5')
 for kind,item,limit in [('part','tconstruct:tool_handle',8),('tinkers','minecraft:diamond',16)]:
  cmd('clear @s');cmd('setblock 0 71 2 minecraft:air');cmd('setblock 0 71 2 tconstruct:'+('part_chest' if kind=='part' else 'tinkers_chest'))
  give(item+'[minecraft:custom_data={proof:17}]',4);opench();s=hot(item);click(s);click(300);assert cells().get(300,{}).get('count')==4,cells();g.call('close')
  give(item+'[minecraft:custom_data={proof:17}]',limit);opench();s=hot(item);click(s);click(300);assert cells().get(300,{}).get('count')==limit,cells();click(s);g.call('close');inv=g.call('inventory');rest=next(x for x in inv['hotbar'] if x.get('item')==item);assert rest['count']==4,rest;cmd('clear @s')
  give(item+'[minecraft:custom_data={proof:18}]',2);opench();click(hot(item));click(301);c=cells();assert c[301]['count']==2 and 'proof:18' in c[301].get('components',''),c;assert 'proof:17' in c[300].get('components',''),c;g.call('close')
  if kind=='part':
   give('minecraft:diamond',1);opench();s=hot('minecraft:diamond');click(s);click(302);assert 302 not in cells();click(s);g.call('close');assert any(x.get('item')=='minecraft:diamond' for x in g.call('inventory')['hotbar'])
  g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',resume=True,in_frame=True);g.ready();opench();c=cells();assert c[300]['count']==limit and 'proof:17' in c[300].get('components',''),c;assert c[301]['count']==2 and 'proof:18' in c[301].get('components',''),c;print('PASS',kind,'same/different component partial remainder rejection save/reload',c[300],c[301],flush=True);g.call('close')
 g.stop()
finally:g.cleanup()

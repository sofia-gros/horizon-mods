import sys,tempfile,time,argparse,json
from pathlib import Path
from types import SimpleNamespace
from concurrent.futures import ThreadPoolExecutor
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true');p.add_argument('--source-choice-wire-test',action='store_true');p.add_argument('--flint-steel',action='store_true');p.add_argument('--source-gui-test',action='store_true');p.add_argument('--output-parent',type=Path,default=Path('target'))
a=p.parse_args();a.output_parent.mkdir(parents=True,exist_ok=True);run=Path(tempfile.mkdtemp(prefix='recycling-race-',dir=a.output_parent.resolve()))
print('run',run,flush=True)
games=[]
def click(g,s): g.call('click-slot',str(s));g.call('wait','3')
def slots(g): return {s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
def wait(label,pred):
 for _ in range(100):
  if pred(): print('ok',label,flush=True);return
  time.sleep(.2)
 raise AssertionError(label)
def pattern(g,label):
 def node():
  return next((n for n in g.call('ui')['widgets'] if n.get('label')==label and n.get('enabled')),None)
 wait(g.label+' pattern '+label,lambda:node() is not None)
 g.call('click',node()['id']);g.call('wait','3')
try:
 h=Game(a,run,'Host',in_frame=a.in_frame);games.append(h);h.ready()
 port=h.call('lan','open','0','commands')['port']
 q=Game(a,run,'Guest',server='127.0.0.1:'+str(port));games.append(q);q.ready()
 for cmd in ('gamemode creative @a','clear @a','tp @s -0.5 71 0.5','tp Guest 1.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','give @s tconstruct:venom_bottle 1', ('give @s minecraft:flint_and_steel 1' if a.flint_steel else 'give @s minecraft:stone_axe 1')):
  h.call('command',cmd)
 for g in games:
  wait(g.label+' block',lambda:g.call('block','0','71','2').get('block')=='tconstruct:part_builder')
  g.call('interact','0','71','2')
 click(h,0);click(h,300);click(h,1);click(h,301)
 wait('inputs shared',lambda:all(slots(g).get(300,{}).get('item')=='tconstruct:venom_bottle' and slots(g).get(301,{}).get('item')==('minecraft:flint_and_steel' if a.flint_steel else 'minecraft:stone_axe') for g in games))
 pattern(q,'Shard' if a.flint_steel else 'Block')
 h.call('wait','20');q.call('wait','20');time.sleep(2)
 with ThreadPoolExecutor(max_workers=2) as pool:
  futures=[pool.submit(click,g,322) for g in games]
  for f in futures: f.result()
 item = 'minecraft:flint' if a.flint_steel else 'minecraft:cobblestone'
 expected = 1 if a.flint_steel else 3
 total=sum((g.call('menu').get('carried') or {}).get('count',0) for g in games if (g.call('menu').get('carried') or {}).get('item')==item)
 print('recycled output',total,flush=True)
 assert total==expected, 'simultaneous pickup must recycle the one input exactly once'
 if a.flint_steel:
  iron=sum(x.get('count',0) for g in games for x in g.call('inventory')['hotbar'] if x.get('item')=='minecraft:iron_ingot')
  assert iron==1, ('exactly one random leftover',iron)

 wait('glass bottle returned',lambda:slots(h).get(300,{}).get('item')=='minecraft:glass_bottle' and 301 not in slots(h))
 for g in reversed(games):g.stop()
finally:
 for g in reversed(games):g.cleanup()

import sys,tempfile,time,argparse,json
from pathlib import Path
from types import SimpleNamespace
from concurrent.futures import ThreadPoolExecutor
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true');p.add_argument('--right',action='store_true');p.add_argument('--source-choice-wire-test',action='store_true');p.add_argument('--source-gui-test',action='store_true');p.add_argument('--output-parent',type=Path,default=Path('target'))
a=p.parse_args();a.output_parent.mkdir(parents=True,exist_ok=True);run=Path(tempfile.mkdtemp(prefix='source-quickmove-race-',dir=a.output_parent.resolve()))
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
 for cmd in ('gamemode creative @a','clear @a','tp @s -0.5 71 0.5','tp Guest 1.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','give @s tconstruct:pattern 2','give @s minecraft:oak_log 1'):
  h.call('command',cmd)
 for g in games:
  wait(g.label+' block',lambda:g.call('block','0','71','2').get('block')=='tconstruct:part_builder')
  g.call('interact','0','71','2')
 click(h,0);click(h,300);click(h,1);click(h,301)
 wait('inputs shared',lambda:all(slots(g).get(300,{}).get('count')==2 and slots(g).get(301,{}).get('count')==1 for g in games))
 pattern(q,'Tool Handle')
 h.call('wait','20');q.call('wait','20');time.sleep(2)
 def quick(g): g.call('click-slot','322',*(['right'] if a.right else []),'shift');g.call('wait','10')
 with ThreadPoolExecutor(max_workers=2) as pool:
  futures=[pool.submit(quick,g) for g in games]
  for f in futures: f.result()
 total=0
 for g in games:
  inv=g.call('inventory')['hotbar']
  total+=sum(s.get('count',0) for s in inv if s.get('item')=='tconstruct:tool_handle')
 print('crafted handles',total,flush=True)
 assert total==1, 'simultaneous quick moves must consume one log once'
 assert slots(h).get(300,{}).get('count')==1 and 301 not in slots(h),slots(h)
 for g in reversed(games):g.stop()
finally:
 for g in reversed(games):g.cleanup()

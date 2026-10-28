import sys,tempfile,time,argparse
from pathlib import Path
from types import SimpleNamespace
from concurrent.futures import ThreadPoolExecutor
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true');p.add_argument('--output-parent',type=Path,default=Path('target'))
a=p.parse_args();a.output_parent.mkdir(parents=True,exist_ok=True);run=Path(tempfile.mkdtemp(prefix='station-race-',dir=a.output_parent.resolve()))
print('run',run,flush=True)
games=[]
def click(g,s): g.call('click-slot',str(s));g.call('wait','3')
def slots(g): return {s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
def wait(label,pred):
 for _ in range(100):
  if pred(): print('ok',label,flush=True);return
  time.sleep(.2)
 raise AssertionError(label)
try:
 h=Game(a,run,'Host',in_frame=a.in_frame);games.append(h);h.ready()
 port=h.call('lan','open','0','commands')['port']
 q=Game(a,run,'Guest',server='127.0.0.1:'+str(port));games.append(q);q.ready()
 for cmd in ('gamemode creative @a','clear @a','tp @s -0.5 71 0.5','tp Guest 1.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','give @s tconstruct:pattern 1','give @s minecraft:flint 2'):
  print('command',cmd,h.call('command',cmd),flush=True)
 for g in games:
  wait(g.label+' block',lambda:g.call('block','0','71','2').get('block')=='tconstruct:part_builder')
  print(g.label,'interact',g.call('interact','0','71','2'),flush=True)
 click(h,0);click(h,300);click(h,1);click(h,301)
 wait('inputs shared',lambda:all(slots(g).get(300,{}).get('item')=='tconstruct:pattern' and slots(g).get(301,{}).get('count')==2 for g in games))
 click(h,324);assert not (h.call('menu').get('carried') or {})
 click(q,325);q.call('wait','20');h.call('wait','20');click(h,322)
 assert not (h.call('menu').get('carried') or {}), 'guest selection must replace host selection'
 click(q,324)
 h.call('wait','20');q.call('wait','20');time.sleep(2)
 h.call('wait','20');q.call('wait','20');time.sleep(2)
 h.call('screenshot',str(run/'host-selected.png'));q.call('screenshot',str(run/'guest-selected.png'))
 with ThreadPoolExecutor(max_workers=2) as pool:
  futures=[pool.submit(click,g,322) for g in games]
  for f in futures:f.result()
 for g in games: print(g.label,'result',g.call('menu'),flush=True)
 print('outputs',sum(1 for g in games if (g.call('menu').get('carried') or {}).get('item')=='tconstruct:pick_head'),flush=True)
 assert sum(1 for g in games if (g.call('menu').get('carried') or {}).get('item')=='tconstruct:pick_head')==1
 assert all(301 not in slots(g) for g in games)
 for g in reversed(games):g.stop()
finally:
 for g in reversed(games):g.cleanup()

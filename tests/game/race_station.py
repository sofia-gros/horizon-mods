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
 for cmd in ('gamemode creative @a','clear @a','tp @s -0.5 71 0.5','tp Guest 1.5 71 0.5','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"},minecraft:item_model="tconstruct:flint_pick_head"] 1','give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"},minecraft:item_model="tconstruct:wood_handle"] 1','give @s tconstruct:tool_binding[minecraft:custom_data={Material:"tconstruct:wood"},minecraft:item_model="tconstruct:wood_binding"] 1'):
  print('command',cmd,h.call('command',cmd),flush=True)
 for g in games:
  wait(g.label+' block',lambda:g.call('block','0','71','2').get('block')=='tconstruct:tinker_station')
  print(g.label,'interact',g.call('interact','0','71','2'),flush=True)
 click(h,0);click(h,300);click(h,1);click(h,301);click(h,2);click(h,302)
 wait('inputs shared',lambda:all(slots(g).get(300,{}).get('item')=='tconstruct:pick_head' and slots(g).get(301,{}).get('item')=='tconstruct:tool_handle' and slots(g).get(302,{}).get('item')=='tconstruct:tool_binding' for g in games))
 h.call('wait','20');q.call('wait','20')
 h.call('screenshot',str(run/'host-station.png'));q.call('screenshot',str(run/'guest-station.png'))
 with ThreadPoolExecutor(max_workers=2) as pool:
  futures=[pool.submit(click,g,324) for g in games]
  for f in futures:f.result()
 for g in games: print(g.label,'result',g.call('menu'),flush=True)
 print('outputs',sum(1 for g in games if (g.call('menu').get('carried') or {}).get('item')=='tconstruct:pickaxe'),flush=True)
 assert sum(1 for g in games if (g.call('menu').get('carried') or {}).get('item')=='tconstruct:pickaxe')==1
 wait('inputs consumed once',lambda: all(all(slot not in slots(g) for slot in (300,301,302)) for g in games))
 for g in reversed(games):g.stop()
finally:
 for g in reversed(games):g.cleanup()

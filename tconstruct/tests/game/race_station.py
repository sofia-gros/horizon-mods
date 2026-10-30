import sys,time,argparse
from pathlib import Path
from types import SimpleNamespace
from concurrent.futures import ThreadPoolExecutor
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true');p.add_argument('--guest-only',action='store_true');p.add_argument('--builder-transition',action='store_true');p.add_argument('--mattock',action='store_true');p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
a=p.parse_args();run=a.run_dir.resolve();run.mkdir(parents=True,exist_ok=True)
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
 h=Game(a,run,'Player',in_frame=a.in_frame,resume=(run/'Player').exists());games.append(h);h.ready()
 port=h.call('lan','open','0','commands')['port']
 q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=(run/'Guest').exists());games.append(q);q.ready()
 if a.builder_transition:
  for cmd in ('gamemode creative @a','clear @a','tp @s -0.5 71 0.5','tp Guest 1.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:part_builder','give Guest tconstruct:pattern 1','give Guest minecraft:flint 1'):
   h.call('command',cmd)
  wait('guest builder',lambda:q.call('block','0','71','2').get('block')=='tconstruct:part_builder')
  q.call('interact','0','71','2');q.call('wait','10')
  def guest_hotbar(item): return next(s['slot'] for s in q.call('inventory')['hotbar'] if s.get('item')==item)
  click(q,guest_hotbar('tconstruct:pattern'));click(q,300);click(q,guest_hotbar('minecraft:flint'));click(q,301)
  wait('guest pattern choice',lambda:any(w.get('label')=='Pick Head' and w.get('enabled') for w in q.call('ui')['widgets']))
  q.call('click',next(w['id'] for w in q.call('ui')['widgets'] if w.get('label')=='Pick Head' and w.get('enabled')));q.call('wait','5');q.call('close')
 part_ids=('tconstruct:small_axe_head','tconstruct:tool_handle','tconstruct:adze_head') if a.mattock else ('tconstruct:pick_head','tconstruct:tool_handle','tconstruct:tool_binding')
 materials=('flint','wood','flint') if a.mattock else ('flint','wood','wood')
 result='tconstruct:mattock' if a.mattock else 'tconstruct:pickaxe'
 commands=('gamemode creative @a','clear @a','tp @s -0.5 71 0.5','tp Guest 1.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:tinker_station')
 commands += tuple(f'give @s {part}[minecraft:custom_data={{Material:"tconstruct:{material}"}}] 1' for part,material in zip(part_ids,materials))
 for cmd in commands:
  print('command',cmd,h.call('command',cmd),flush=True)
 for g in games:
  wait(g.label+' block',lambda:g.call('block','0','71','2').get('block')=='tconstruct:tinker_station')
  print(g.label,'interact',g.call('interact','0','71','2'),flush=True)
 if a.mattock: q.call('click-gui','-14','52');q.call('wait','5')
 click(h,0);click(h,300);click(h,1);click(h,301);click(h,2);click(h,302)
 wait('inputs shared',lambda:all(all(slots(g).get(300+i,{}).get('item')==part for i,part in enumerate(part_ids)) for g in games))
 wait('guest result preview',lambda:slots(q).get(324,{}).get('item')==result)
 if a.mattock:
  q.call('click-gui','-14','16');q.call('wait','5');click(q,324)
  assert q.call('menu').get('carried') is None, 'wrong layout must reject the result'
  q.call('click-gui','-14','52');q.call('wait','5')
 h.call('wait','20');q.call('wait','20')
 h.call('screenshot',str(run/'host-station.png'));q.call('screenshot',str(run/'guest-station.png'))
 with ThreadPoolExecutor(max_workers=2) as pool:
  futures=[pool.submit(click,g,324) for g in ([q] if a.guest_only else games)]
  for f in futures:f.result()
 for g in games: print(g.label,'result',g.call('menu'),flush=True)
 print('outputs',sum(1 for g in games if (g.call('menu').get('carried') or {}).get('item')==result),flush=True)
 assert sum(1 for g in games if (g.call('menu').get('carried') or {}).get('item')==result)==1
 winner=next(g for g in games if (g.call('menu').get('carried') or {}).get('item')==result)
 if a.mattock:
  empty=next(s['slot'] for s in winner.call('inventory')['hotbar'] if not s.get('item'));click(winner,empty)
  item=next(s for s in winner.call('inventory')['hotbar'] if s.get('item')==result)
  assert 'tic_max_damage:106' in item.get('components',''), item
 observer=next(g for g in games if g is not winner)
 observer.call('close');observer.call('interact','0','71','2')
 wait('observer reopened station',lambda:any(w['id']=='slot-300' for w in observer.call('ui')['widgets']))
 assert all(slot not in slots(observer) for slot in (300,301,302)), 'all parts must be consumed once'
 for g in reversed(games):g.stop()
finally:
 for g in reversed(games):g.cleanup()

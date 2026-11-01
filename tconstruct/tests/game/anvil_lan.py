import argparse,sys,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources','run_dir'):p.add_argument('--'+k.replace('_','-'),type=Path,required=True)
a=p.parse_args()
if not a.run_dir.resolve().is_relative_to((Path(__file__).resolve().parents[2] / 'target').resolve()):
 raise SystemExit('run-dir must be inside tconstruct/target')
games=[]
def wait(label,pred):
 for _ in range(120):
  if pred():return
  time.sleep(.2)
 raise AssertionError(label)
def click(g,slot,*flags):g.call('click-slot',str(slot),*flags);g.call('wait','5')
def hot(g,item):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
try:
 h=Game(a,a.run_dir,'Player',resume=True);games.append(h);h.ready();port=h.call('lan','open','0','commands')['port']
 q=Game(a,a.run_dir,'Guest',server='127.0.0.1:'+str(port),resume=(a.run_dir/'Guest').exists());games.append(q);q.ready()
 for c in ('gamemode creative @a','clear Guest','tp @s -2.5 71 0.5','tp Guest 0.5 71 0.5', 'give Guest tconstruct:hammer_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1', 'give Guest tconstruct:tough_handle[minecraft:custom_data={Material:"tconstruct:flint"}] 1', 'give Guest tconstruct:large_plate[minecraft:custom_data={Material:"tconstruct:flint"}] 2'):
  h.call('command',c)
 h.call('wait','20');wait('guest anvil',lambda:q.call('block','1','71','2').get('block')=='tconstruct:tinkers_anvil')
 q.call('look-at','block','1','71','2');q.call('wait','20');q.call('screenshot',str(a.run_dir/'anvil-lan-world.png'));q.call('interact','1','71','2');q.call('wait','20')
 def layout(): return next((w for w in q.call('ui')['widgets'] if w.get('label')=='Sledge Hammer' and w.get('enabled')),None)
 wait('sledge layout',lambda:layout() is not None);q.call('click',layout()['id']);q.call('wait','10')
 click(q,hot(q,'tconstruct:hammer_head'));click(q,300);click(q,hot(q,'tconstruct:tough_handle'));click(q,301)
 click(q,hot(q,'tconstruct:large_plate'));click(q,302,'right');click(q,303)
 def output():return next((row['stack'] for row in q.call('menu')['slots'] if row.get('slot')==324 and row.get('stack')),None)
 wait('sledge preview',lambda:(output() or {}).get('item')=='tconstruct:sledge_hammer')
 q.call('screenshot',str(a.run_dir/'anvil-lan-assembly.png'));click(q,324)
 result=q.call('menu')['carried'] or {};assert result.get('item')=='tconstruct:sledge_hammer',result
 click(q,0);q.call('close')
 h.call('command','give Guest tconstruct:large_plate[minecraft:custom_data={Material:"tconstruct:wood"}] 1');h.call('wait','10')
 q.call('interact','1','71','2');q.call('wait','15')
 click(q,hot(q,'tconstruct:sledge_hammer'));click(q,300);click(q,hot(q,'tconstruct:large_plate'));click(q,305)
 def operation():return next((row['stack'] for row in q.call('menu')['slots'] if row.get('slot')==325 and row.get('stack')),None)
 wait('part exchange preview',lambda:(operation() or {}).get('item')=='tconstruct:sledge_hammer')
 click(q,325);result=q.call('menu')['carried'] or {}
 assert result.get('item')=='tconstruct:sledge_hammer',result
 click(q,0);q.call('close');q.call('wait','10')
 inventory=q.call('inventory')['hotbar'];tool=next((s for s in inventory if s.get('item')=='tconstruct:sledge_hammer'),None)
 assert tool and 'tconstruct:wood' in tool.get('components',''),tool
 h.call('save')
 for game in reversed(games):game.stop()
 print('PASS LAN anvil assembly')
finally:
 for game in reversed(games):game.cleanup()

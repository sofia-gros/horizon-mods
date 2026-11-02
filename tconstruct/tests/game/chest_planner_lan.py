import argparse,sys,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'): p.add_argument('--'+n,type=Path,required=True)
a=p.parse_args(); run=a.run_dir.resolve(); assert run.is_relative_to(Path(__file__).resolve().parents[2] / 'target'); games=[]
def cmd(g,s): g.call('command',s);g.call('wait','6')
def click(g,n): g.call('click-slot',str(n));g.call('wait','8')
def cells(g): return {int(w['id'][5:]):w['stack'] for w in g.call('ui')['widgets'] if w['id'].startswith('slot-') and w.get('stack')}
def carried(g): return g.call('menu').get('carried') or {}
def hot(g,item):
 for _ in range(40):
  inv=g.call('inventory'); matches=[x['slot'] for x in inv['hotbar'] if x.get('item')==item]
  if matches:return matches[0]
  time.sleep(.25)
 print('MISSING HOTBAR',item,inv,flush=True);raise AssertionError('guest item not present')
def wait(pred,label):
 for _ in range(80):
  if pred():return
  time.sleep(.2)
 print('SYNC DEBUG',label,'host',cells(h),'guest',cells(q),'guest inventory',q.call('inventory'),flush=True);raise AssertionError(label)
def give(h,item,n): cmd(h,'give Guest '+item+' '+str(n))
def open_chest(g):g.call('select','9');g.call('interact','0','71','2');g.call('wait','10')
def put(g,item,index):
 s=hot(g,item);click(g,s);click(g,index);return s
try:
 h=Game(a,run,'Player',resume=True,in_frame=True);games.append(h);h.ready();port=h.call('lan','open','0','commands')['port']
 q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=True,in_frame=True);games.append(q);q.ready()
 for c in ('gamemode creative @a','clear @a','tp @s -0.5 71 0.5','tp Guest 1.5 71 0.5','fill -2 70 -2 4 70 4 minecraft:stone'):cmd(h,c)
 for kind,item,limit in [('part','tconstruct:tool_handle',8),('tinkers','minecraft:diamond',16)]:
  cmd(h,'setblock 0 71 2 minecraft:air');cmd(h,'setblock 0 71 2 tconstruct:'+('part_chest' if kind=='part' else 'tinkers_chest'));cmd(h,'clear Guest')
  give(h,item+'[minecraft:custom_data={proof:17}]',4);open_chest(h);open_chest(q);put(q,item,300)
  wait(lambda:cells(h).get(300,{}).get('count')==4,kind+' guest initial sync');assert 'proof:17' in cells(q)[300].get('components','');q.call('close');h.call('close')
  give(h,item+'[minecraft:custom_data={proof:17}]',limit);open_chest(h);open_chest(q);s=put(q,item,300)
  wait(lambda:cells(h).get(300,{}).get('count')==limit,kind+' partial merge sync'); assert carried(q).get('count')==4,(kind,carried(q));click(q,s);q.call('close');h.call('close');cmd(h,'clear Guest')
  give(h,item+'[minecraft:custom_data={proof:18}]',2);open_chest(h);open_chest(q);put(q,item,301)
  wait(lambda:cells(h).get(301,{}).get('count')==2,kind+' different component sync');assert 'proof:18' in cells(q)[301].get('components','');assert 'proof:17' in cells(q)[300].get('components','')
  q.call('close');h.call('close')
  if kind=='part':
   give(h,'minecraft:diamond',1);open_chest(q);s=put(q,'minecraft:diamond',302);assert carried(q).get('item')=='minecraft:diamond';assert 302 not in cells(q);click(q,s);q.call('close')
  h.call('save');q.stop();q.cleanup();games.remove(q);q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=True,in_frame=True);games.append(q);q.ready();cmd(h,'tp Guest 1.5 71 0.5');open_chest(q)
  c=cells(q);assert c[300]['count']==limit and 'proof:17' in c[300].get('components',''),c;assert c[301]['count']==2 and 'proof:18' in c[301].get('components',''),c
  print('PASS',kind,'LAN initial/merge/partial remainder/identity/reconnect',c[300],c[301],flush=True);q.call('close')
  if kind=='part':
   q.stop();q.cleanup();games.remove(q);h.call('save');h.stop();h.cleanup();games.remove(h)
   h=Game(a,run,'Player',resume=True,in_frame=True);games.append(h);h.ready();open_chest(h);saved=cells(h);assert saved[300]['count']==8 and 'proof:17' in saved[300].get('components','');assert saved[301]['count']==2 and 'proof:18' in saved[301].get('components','');print('PASS part save/reload',saved[300],saved[301],flush=True);h.call('close');port=h.call('lan','open','0','commands')['port'];q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=True,in_frame=True);games.append(q);q.ready();cmd(h,'tp Guest 1.5 71 0.5')
 q.stop();q.cleanup();games.remove(q);h.call('save');h.stop();h.cleanup();games.remove(h)
 h=Game(a,run,'Player',resume=True,in_frame=True);games.append(h);h.ready();open_chest(h);c=cells(h);assert c[300]['count']==16 and 'proof:17' in c[300].get('components','');assert c[301]['count']==2 and 'proof:18' in c[301].get('components','');print('PASS tinkers save/reload',c[300],c[301],flush=True);h.call('close');h.stop()
finally:
 for g in reversed(games):g.cleanup()

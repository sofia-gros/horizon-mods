import sys,types,pathlib,json,time
sys.path.insert(0,'/workspace/scratch/ui-pr265-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path(sys.argv[1]);client=pathlib.Path(sys.argv[2]);ctl=pathlib.Path(sys.argv[3]);res=pathlib.Path(sys.argv[4]);a=types.SimpleNamespace(game=client,ctl=ctl,resources=res);games=[]
def hover(g,where,name):
 w=next(x for x in g.call('ui')['widgets'] if x.get('id')=='slot-300');x,y,_,_=w['bounds'];dx,dy=(8,8) if where=='slot' else (76,45);g.call('scroll-at',str(x+dx),str(y+dy),'0');g.call('wait','10');g.call('screenshot',str(root/name));print(name,'slot',w,flush=True)
def openmelter(g):
 print('OPEN',json.dumps(g.call('interact','0','72','2')),flush=True);g.call('wait','15');assert any(x.get('id')=='slot-300' for x in g.call('ui')['widgets'])
try:
 h=Game(a,root,'Player',resume=True);games.append(h);h.ready();h.call('command','gamemode creative @s');h.call('command','tp @s 2.5 71 1.5');h.call('wait','10');print('BE_BEFORE',json.dumps(h.call('command','data get block 0 72 2')),flush=True);openmelter(h);hover(h,'slot','local-slot.png');hover(h,'tank','local-tank.png');h.call('close');openmelter(h);hover(h,'tank','local-reopen.png');h.call('close')
 port=h.call('lan','open','0','commands')['port'];q=Game(a,root,'Guest',server='127.0.0.1:'+str(port));games.append(q);q.ready();h.call('command','gamemode creative @a');h.call('command','clear Guest');h.call('command','tp Guest 2.5 71 1.5');h.call('wait','15');q.call('hotbar','1');q.call('wait','5')
 for _ in range(100):
  if q.call('block','0','72','2').get('block')=='tconstruct:seared_melter':break
  time.sleep(.2)
 openmelter(q);hover(q,'slot','remote-slot.png');hover(q,'tank','remote-tank.png');q.call('close');openmelter(q);hover(q,'tank','remote-reopen.png');q.call('close');print('BE_AFTER',json.dumps(h.call('command','data get block 0 72 2')),flush=True)
 h.call('command','setblock 0 71 3 minecraft:chest');h.call('wait','10');q.call('interact','0','71','3');q.call('wait','10');q.call('screenshot',str(root/'remote-chest.png'));print('CHEST',json.dumps(q.call('menu')),flush=True);q.call('close');q.call('open-inventory');q.call('wait','10');q.call('screenshot',str(root/'remote-inventory.png'));print('INVENTORY',json.dumps(q.call('menu')),flush=True);q.call('close');h.call('save')
 for g in reversed(games):g.stop()
 print('DONE capture local and remote Melter, chest and inventory',flush=True)
finally:
 for g in reversed(games):g.cleanup()

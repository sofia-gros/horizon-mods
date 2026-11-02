import sys,types,pathlib,json,time
sys.path.insert(0,'/workspace/scratch/ui-melter-pointer-pr/tconstruct/tests/game');from harness import Game
root=pathlib.Path(sys.argv[1]);a=types.SimpleNamespace(game=pathlib.Path(sys.argv[2]),ctl=pathlib.Path(sys.argv[3]),resources=pathlib.Path(sys.argv[4]));games=[]
def wait_open(g):
 print('OPEN',json.dumps(g.call('interact','0','72','2')),flush=True)
 for _ in range(100):
  if any(x.get('id')=='slot-300' for x in g.call('ui')['widgets']):return
  time.sleep(.2)
 raise AssertionError('Melter menu unavailable')
def extract(g,label):
 wait_open(g);slot=next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:copper_can');g.call('click-slot',str(slot));g.call('wait','8');g.call('screenshot',str(root/(label+'-before.png')));g.call('click-gui','100','60');g.call('wait','15');print('MENU',label,json.dumps(g.call('menu')),flush=True);g.call('screenshot',str(root/(label+'-after.png')));g.call('click-slot',str(slot));g.call('close');g.call('hotbar',str(slot+1));g.call('wait','8');held=g.call('inventory')['held'];print('HELD',label,json.dumps(held),flush=True);assert held.get('item')=='tconstruct:copper_can' and 'tconstruct:molten_copper' in held.get('components',''),held
def deposit(g,label):
 wait_open(g);slot=next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:copper_can');g.call('click-slot',str(slot));g.call('wait','8');g.call('click-gui','100','60','right');g.call('wait','15');print('DEPOSIT_MENU',label,json.dumps(g.call('menu')),flush=True);g.call('screenshot',str(root/(label+'-deposit.png')));g.call('click-slot',str(slot));g.call('close');g.call('hotbar',str(slot+1));g.call('wait','8');held=g.call('inventory')['held'];print('EMPTY_CAN',label,json.dumps(held),flush=True);assert held.get('item')=='tconstruct:copper_can' and 'molten_copper' not in held.get('components',''),held
try:
 h=Game(a,root,'Player',resume=True);games.append(h);h.ready()
 for c in ['gamemode creative @s','tick rate 20','setblock 2 70 1 minecraft:stone','tp @s 2.5 71 1.5','clear @s','give @s tconstruct:copper_can 1']:h.call('command',c)
 h.call('wait','10');extract(h,'local');deposit(h,'local');port=h.call('lan','open','0','commands')['port'];q=Game(a,root,'Guest',server='127.0.0.1:'+str(port));games.append(q);q.ready()
 for c in ['gamemode creative @a','tp @s -2.5 71 0.5','clear Guest','tp Guest 2.5 71 1.5','give Guest tconstruct:copper_can 1']:h.call('command',c)
 h.call('wait','15');q.call('hotbar','1');q.call('wait','5')
 for _ in range(100):
  if q.call('block','0','72','2').get('block')=='tconstruct:seared_melter':break
  time.sleep(.2)
 extract(q,'guest');deposit(q,'guest');h.call('save')
 for g in reversed(games):g.stop()
 print('PASS diagnostic local and guest can fill and deposit',flush=True)
finally:
 for g in reversed(games):g.cleanup()

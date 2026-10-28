import argparse,tempfile,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true')
a=p.parse_args();root=Path(tempfile.mkdtemp(prefix='material-wire-self-',dir=Path('target').resolve()));print('run',root,flush=True);games=[]
def wait(label,fn):
 for _ in range(150):
  if fn(): print('ok',label,flush=True);return
  time.sleep(.2)
 raise AssertionError(label)
def count(game,material):
 text=(game.root/'game.log').read_text()
 return sum('TCONSTRUCT_WIRE_OBSERVED 0 71 2 '+material in line for line in text.splitlines())
def stored_count(game,material):
 text=(game.root/'game.log').read_text()
 return sum('TCONSTRUCT_WIRE_STORED '+material in line for line in text.splitlines())
def check_stored(game,material):
 before=stored_count(game,material)
 game.call('interact','2','71','2')
 wait('client stored '+material,lambda:stored_count(game,material)>before)
 game.call('close')
def place(game,material):
 game.call('command','clear @s')
 item = f'tconstruct:fake_storage_block[minecraft:custom_data={{Material:"{material}"}}]' if material is not None else 'tconstruct:fake_storage_block'
 game.call('command',f'give @s {item} 1')
 wait('host item '+str(material),lambda:any(s.get('item')=='tconstruct:fake_storage_block' for s in game.call('inventory')['hotbar']))
 game.call('place','0','71','2','--item','tconstruct:fake_storage_block')
try:
 h=Game(a,root,'Host',in_frame=a.in_frame);games.append(h);h.ready()
 port=h.call('lan','open','0','commands')['port']
 q=Game(a,root,'Guest',server='127.0.0.1:'+str(port));games.append(q);q.ready()
 for cmd in ('fill -2 70 -1 4 70 5 minecraft:stone','gamemode creative @a','tp @s -0.5 71 0.5','tp Guest 1.5 71 0.5','setblock 2 71 2 minecraft:chest'):h.call('command',cmd)
 wait('host support',lambda:h.call('block','0','70','2').get('block')=='minecraft:stone')
 place(h,'tconstruct:bronze')
 wait('guest block',lambda:q.call('block','0','71','2').get('block')=='tconstruct:fake_storage_block')
 wait('guest bronze',lambda:count(q,'tconstruct:bronze')>=1)
 wait('guest chest',lambda:q.call('block','2','71','2').get('block')=='minecraft:chest')
 check_stored(q,'tconstruct:bronze')
 check_stored(h,'tconstruct:bronze')
 if not a.in_frame: wait('host bronze',lambda:count(h,'tconstruct:bronze')>=1)
 before=count(q,'tconstruct:bronze');q.call('wait','100');time.sleep(1)
 assert count(q,'tconstruct:bronze')==before, 'unchanged material resent'
 print('ok no repeat',flush=True)
 h.call('command','setblock 0 71 2 minecraft:air')
 wait('guest removed',lambda:q.call('block','0','71','2').get('block')=='minecraft:air')
 h.call('wait','20');place(h,'tconstruct:unknown')
 wait('guest unknown',lambda:count(q,'tconstruct:unknown')>=1)
 check_stored(q,'tconstruct:unknown')
 check_stored(h,'tconstruct:unknown')
 if not a.in_frame: wait('host unknown',lambda:count(h,'tconstruct:unknown')>=1)
 previous=count(q,'tconstruct:unknown');q.stop();q.cleanup();games.remove(q);h.call('wait','20')
 q=Game(a,root,'Guest',server='127.0.0.1:'+str(port),resume=True);games.append(q);q.ready()
 wait('guest reconnect unknown',lambda:count(q,'tconstruct:unknown')>previous)
 check_stored(q,'tconstruct:unknown')
 q.stop();q.cleanup();games.remove(q);h.stop();h.cleanup();games.remove(h)
 h=Game(a,root,'Host',in_frame=a.in_frame,resume=True);games.append(h);h.ready()
 port=h.call('lan','open','0','commands')['port']
 previous=count(q,'tconstruct:unknown')
 q=Game(a,root,'Guest',server='127.0.0.1:'+str(port),resume=True);games.append(q);q.ready()
 wait('saved unknown block',lambda:q.call('block','0','71','2').get('block')=='tconstruct:fake_storage_block')
 wait('saved unknown packet',lambda:count(q,'tconstruct:unknown')>previous)
 check_stored(q,'tconstruct:unknown')
 h.call('command','setblock 0 71 2 minecraft:air')
 wait('guest removed again',lambda:q.call('block','0','71','2').get('block')=='minecraft:air')
 h.call('wait','20');place(h,None)
 wait('guest untagged block',lambda:q.call('block','0','71','2').get('block')=='tconstruct:fake_storage_block')
 check_stored(q,'<none>')
 check_stored(h,'<none>')
 for g in reversed(games):g.stop()
finally:
 for g in reversed(games):g.cleanup()

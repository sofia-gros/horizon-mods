"""Check source bottle refill and remainders for a LAN crafting-table guest."""
import argparse,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
games=[]
def cmd(game,value):game.call('command',value);game.call('wait','8')
def click(game,slot,*extra):game.call('click-slot',str(slot),*extra);game.call('wait','8')
def hot(game,item):return next(x['slot'] for x in game.call('inventory')['hotbar'] if x.get('item')==item)
try:
 h=Game(a,run,'Player',in_frame=a.in_frame,resume=True);games.append(h);h.ready()
 port=h.call('lan','open','0','commands')['port']
 q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=(run/'Guest').exists());games.append(q);q.ready()
 for value in ('gamemode creative @a','clear Guest','tp Guest 1.5 71 0.5','tp @s -0.5 71 0.5',
               'setblock 0 71 2 minecraft:crafting_table',
               'give Guest tconstruct:pickaxe[minecraft:max_damage=85,minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85,tic_overslime:30,tic_overslime_capacity:50,tic_overslime_active:1b,tic_upgrades:{"tconstruct:overslime":1}}] 1',
               'give Guest tconstruct:earth_slime_bottle 2','gamemode survival Guest'):cmd(h,value)
 q.call('wait','20');q.call('interact','0','71','2');q.call('wait','20')
 click(q,hot(q,'tconstruct:pickaxe'));click(q,100)
 click(q,hot(q,'tconstruct:earth_slime_bottle'));click(q,101,'right');click(q,102)
 click(q,200,'shift');q.call('close');q.call('wait','20')
 inv=q.call('inventory');print('LAN CRAFT',inv,flush=True)
 item=next(x for x in inv['hotbar']+inv['main'] if x.get('item')=='tconstruct:pickaxe')
 assert 'tic_overslime:50' in item.get('components',''),item
 assert inv['counts'].get('minecraft:glass_bottle',0)==1 and inv['counts'].get('tconstruct:earth_slime_bottle',0)==1,inv
 h.call('save')
 for game in reversed(games):game.stop()
 print('PASS LAN shift bottle craft and remainders',flush=True)
finally:
 for game in reversed(games):game.cleanup()

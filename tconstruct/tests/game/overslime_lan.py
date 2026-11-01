"""Check supplied shield state on a LAN guest with two armor pieces."""
import argparse,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
games=[]
def cmd(game,value):game.call('command',value);game.call('wait','8')
def data():return 'minecraft:max_damage=100,minecraft:custom_data={tic_materials:["tconstruct:iron","tconstruct:iron"],tic_max_damage:100,tic_overslime:20,tic_overslime_capacity:50,tic_overslime_active:1b,tic_upgrades:{"tconstruct:overslime":1,"tconstruct:unbreakable":1}}'
try:
 h=Game(a,run,'Player',in_frame=a.in_frame,resume=True);games.append(h);h.ready()
 port=h.call('lan','open','0','commands')['port']
 q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=(run/'Guest').exists());games.append(q);q.ready()
 for value in ('gamemode creative @a','clear Guest','effect clear Guest','effect give Guest minecraft:instant_health 1 4 true',
               'give Guest tconstruct:plate_helmet['+data()+'] 1',
               'give Guest tconstruct:plate_chestplate['+data()+'] 1'):cmd(h,value)
 cmd(h,'gamemode survival Guest')
 q.call('wait','15');q.call('open-inventory');q.call('wait','10')
 for item,slot in [('tconstruct:plate_helmet',500),('tconstruct:plate_chestplate',501)]:
  hot=next(x['slot'] for x in q.call('inventory')['hotbar'] if x.get('item')==item)
  q.call('click-slot',str(hot));q.call('wait','5');q.call('click-slot',str(slot));q.call('wait','5')
 q.call('close');q.call('wait','10')
 print('GUEST BEFORE DAMAGE',q.call('inventory')['armor'],flush=True)
 q.call('wait','30');cmd(q,'damage @s 4 minecraft:mob_attack')
 q.call('wait','20');armor=q.call('inventory')['armor'];print('LAN ARMOR',armor,flush=True)
 for stack in armor[:2]:
  assert stack and stack.get('damage',0)==0 and 'tic_overslime:19' in stack.get('components',''),armor
 h.call('save')
 for game in reversed(games):game.stop()
 print('PASS LAN multi-armor shield before Unbreakable',flush=True)
finally:
 for game in reversed(games):game.cleanup()

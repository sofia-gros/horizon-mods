"""A fixed pickaxe wears on a living hit and remains when broken."""
import argparse,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true');p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
a=p.parse_args();run=a.run_dir.resolve();run.mkdir(parents=True,exist_ok=True);print('run',run,flush=True)
g=Game(a,run,'Player',in_frame=a.in_frame,resume=(run/'Player').exists())
try:
 g.ready()
 if g.call('status')['screen']=='dead': g.call('respawn');g.call('wait','20')
 for command in ('gamemode creative @s','kill @e[type=minecraft:zombie]','time set day','effect give @s minecraft:resistance 120 255','clear @s','tp @s 0.5 71 0.5','gamemode survival @s',"item replace entity @s weapon.mainhand with tconstruct:pickaxe[minecraft:damage=84,minecraft:custom_name='\"Blade\"',minecraft:custom_data={proof:17,tic_materials:[\"tconstruct:flint\",\"tconstruct:wood\",\"tconstruct:wood\"]}]"):
  g.call('command',command)
 g.call('command','kill @e[type=minecraft:zombie]');g.call('wait','10');g.call('command','summon minecraft:zombie 1.5 71 0.5')
 zombie=None
 for _ in range(30):
  entities=g.call('entities','8','minecraft:zombie')['entities']
  if entities: zombie=max(e['id'] for e in entities);break
  time.sleep(.2)
 assert zombie is not None, 'server-spawned zombie did not reach client'
 g.call('command',"item replace entity @s weapon.mainhand with tconstruct:pickaxe[minecraft:damage=0,minecraft:custom_data={tic_materials:[\"tconstruct:flint\",\"tconstruct:wood\",\"tconstruct:wood\"]}]")
 g.call('wait','30')
 for _ in range(5):
  g.call('attack',str(zombie));g.call('wait','30')
  held=g.call('inventory')['held']
  if held.get('damage',0) > 0: break
 assert held.get('damage') == 2, held
 g.call('command','summon minecraft:zombie 1.5 71 0.5');g.call('wait','10')
 zombie=max(e['id'] for e in g.call('entities','8','minecraft:zombie')['entities'])
 g.call('command',"item replace entity @s weapon.mainhand with tconstruct:pickaxe[minecraft:damage=84,minecraft:custom_name='\"Blade\"',minecraft:custom_data={proof:17,tic_materials:[\"tconstruct:flint\",\"tconstruct:wood\",\"tconstruct:wood\"]}]")
 g.call('wait','30')
 for _ in range(5):
  hit=g.call('attack',str(zombie));print('attack',hit,flush=True);g.call('wait','30')
  if g.call('inventory')['held'].get('damage') == 85: break
 held=g.call('inventory')['held'];print('after attack',held,flush=True)
 assert held.get('item')=='tconstruct:pickaxe' and held.get('damage')==85,held
 assert all(x in held.get('components','') for x in ('Blade','proof','tic_materials','tic_broken')),held
 g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
 held=g.call('inventory')['held'];assert held.get('item')=='tconstruct:pickaxe' and held.get('damage')==85,held
 assert all(x in held.get('components','') for x in ('Blade','proof','tic_materials','tic_broken')),held
 g.stop();print('PASS melee break and save/reload',flush=True)
finally:g.cleanup()

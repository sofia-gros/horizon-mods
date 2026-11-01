"""Find naturally generated slime clusters in each dimension, then use them."""
import argparse,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();a.natural_world=True;a.world_seed=42
run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());world=run/'Sky'
assert (world/'Player/saves/Probe').is_dir(),'requires the existing disposable seed-42 world'
g=Game(a,world,'Player',in_frame=a.in_frame,resume=True);guest=None

def cmd(s):g.call('command',s);g.call('wait','8')
def click(s,*flags):g.call('click-slot',str(s),*flags);g.call('wait','8')
def hot(i):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==i)
def wait_for(label,pred):
 for _ in range(120):
  value=pred()
  if value:return value
  time.sleep(.2)
 raise AssertionError(label)
def in_dimension(dim,x,y,z):
 cmd(f'execute in minecraft:{dim} run tp @s {x+0.5} {y} {z+0.5}');g.call('wait-ready','90',timeout=95);g.call('wait','15')
def find(color,dimension,y,base):
 for i in range(12):
  x=base+(i%4)*64;z=base+(i//4)*64
  in_dimension(dimension,x,y,z)
  result=g.call('find-block',f'tconstruct:{color}_slime_crystal_cluster','64',timeout=90)
  print('SEARCH',dimension,42,x,z,result,flush=True)
  if result['nearest']:
   pos=result['nearest'][0]['pos'];print('NATURAL',color,dimension,42,pos,flush=True);return pos
 raise AssertionError(f'no {color} cluster in the bounded natural search')
def harvest(color,dimension,pos):
 x,y,z=pos;in_dimension(dimension,x,y,z-2)
 block=g.call('block',x,y,z);assert block.get('block')==f'tconstruct:{color}_slime_crystal_cluster',block
 for c in ('gamemode creative @s',f'fill {x-1} {y-1} {z-2} {x+1} {y+2} {z-1} minecraft:air',f'setblock {x} {y-1} {z-1} minecraft:stone',f'tp @s {x+0.5} {y} {z-1.5}','gamemode survival @s'):cmd(c)
 g.call('hotbar',str(hot('minecraft:diamond_pickaxe')+1));g.call('wait','10');g.call('screenshot',str(run/f'{color}-geode-natural.png'))
 before=g.call('inventory')['counts'].get(f'tconstruct:{color}_slime_crystal',0)
 g.call('break',x,y,z,'--timeout','30',timeout=35);g.call('wait','20');cmd(f'tp @s {x+0.5} {y} {z+0.5}');g.call('wait','60')
 count=g.call('inventory')['counts'].get(f'tconstruct:{color}_slime_crystal',0);print('HARVEST',color,before,count,flush=True);assert count-before==4,(color,before,count)
try:
 g.ready();cmd('gamemode creative @s');cmd('gamerule minecraft:spawn_mobs false');cmd('clear @s');cmd('give @s minecraft:diamond_pickaxe 1')
 earth=find('earth','overworld',-32,2048);in_dimension('overworld',earth[0],earth[1],earth[2]-2)
 g.call('save');port=g.call('lan','open','0','commands')['port'];guest=Game(a,world,'Guest',server='127.0.0.1:'+str(port),resume=(world/'Guest').exists());guest.ready()
 cmd(f'tp Guest {earth[0]+0.5} {earth[1]} {earth[2]-1.5}');guest.call('wait','30');state=guest.call('block',*earth);print('LAN',state,flush=True);assert state.get('block')=='tconstruct:earth_slime_crystal_cluster',state
 guest.stop();guest.cleanup();guest=None
 harvest('earth','overworld',earth)
 ichor=find('ichor','the_nether',96,0);harvest('ichor','the_nether',ichor)
 ender=find('ender','the_end',40,2048);harvest('ender','the_end',ender)
 # Reuse the same world. Test fixtures are the builder and blank pattern; the
 # Earthslime input was picked up from the naturally generated cluster.
 in_dimension('overworld',2048,80,2048)
 for c in ('gamemode creative @s','fill 2046 79 2046 2051 79 2051 minecraft:stone','fill 2046 80 2046 2051 82 2051 minecraft:air','tp @s 2048.5 80 2047.5','setblock 2048 80 2049 tconstruct:part_builder','give @s tconstruct:pattern 1'):cmd(c)
 g.call('hotbar','9');g.call('wait','10');g.call('interact','2048','80','2049');g.call('wait','15')
 click(hot('tconstruct:pattern'));click(300);click(hot('tconstruct:earth_slime_crystal'));click(301)
 def arrow():return next((n for n in g.call('ui')['widgets'] if n.get('label')=='Arrow Head' and n.get('enabled')),None)
 node=wait_for('Earthslime Arrow Head option',arrow);g.call('click',node['id']);g.call('wait','15');g.call('screenshot',str(run/'earth-geode-builder.png'));click(322)
 part=g.call('menu').get('carried') or {};print('BUILDER RESULT',part,flush=True);assert part.get('item')=='tconstruct:arrow_head' and 'tconstruct:earthslime' in part.get('components',''),part
 empty=next(s['slot'] for s in g.call('inventory')['hotbar'] if not s.get('item'));click(empty);g.call('close');g.call('save');g.stop();g.cleanup()
 g=Game(a,world,'Player',in_frame=a.in_frame,resume=True);g.ready();inv=g.call('inventory');print('RELOAD',inv['counts'],flush=True)
 assert any(s.get('item')=='tconstruct:arrow_head' and 'tconstruct:earthslime' in s.get('components','') for s in inv['hotbar'])
 for c in ('ichor','ender'):assert inv['counts'].get(f'tconstruct:{c}_slime_crystal',0)>=4,inv['counts']
 g.stop();print('PASS natural Earth, Ichor and Ender geodes, LAN, builder and reload',flush=True)
finally:
 if guest:guest.cleanup()
 g.cleanup()

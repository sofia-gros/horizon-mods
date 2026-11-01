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
 cmd(f'execute in minecraft:{dim} run tp @s {x+0.5} {y} {z+0.5}');g.call('wait-ready','90',timeout=95);g.call('wait','15');state=g.call('state');assert state.get('dimension')=='minecraft:'+dim,state
# Preselected seed-42 chunk areas where the game's decoration RNG contains
# rarity-filter candidates across possible feature indices. Actual biome,
# geometry and generated clusters are still checked in the running world.
CANDIDATES={
 'earth': [(3968,3200),(3968,3712),(2432,2688),(2048,2176),(3072,2176),(3840,3584),(3712,3840),(2304,2048),(3712,2304),(2688,2688),(2560,2816),(2304,2944)],
 'ichor': [(1408,1792),(256,384),(1664,1152),(768,1664),(1024,1664),(1152,512),(0,1920),(1664,1920),(1792,640),(384,1408),(1536,1664),(384,1792)],
 'ender': [(3968,3200),(3968,3712),(2432,2688),(2048,2176),(3072,2176),(3840,3584),(3712,3840),(2304,2048),(3712,2304),(2688,2688),(2560,2816),(2304,2944)],
}
def find(color,dimension,y):
 for x,z in CANDIDATES[color]:
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
 earth=find('earth','overworld',-32);in_dimension('overworld',earth[0],earth[1],earth[2]-2)
 g.call('save');port=g.call('lan','open','0','commands')['port'];guest=Game(a,world,'Guest',server='127.0.0.1:'+str(port),resume=(world/'Guest').exists());guest.ready()
 cmd(f'tp Guest {earth[0]+0.5} {earth[1]} {earth[2]-1.5}');guest.call('wait','30');state=guest.call('block',*earth);print('LAN',state,flush=True);assert state.get('block')=='tconstruct:earth_slime_crystal_cluster',state
 guest.stop();guest.cleanup();guest=None
 harvest('earth','overworld',earth)
 ichor=find('ichor','the_nether',96);harvest('ichor','the_nether',ichor)
 ender=find('ender','the_end',40);harvest('ender','the_end',ender)
 # Reuse the same world. Test fixtures are the builder and blank pattern; the
 # Earthslime input was picked up from the naturally generated cluster.
 in_dimension('overworld',2048,80,2048)
 for c in ('gamemode creative @s','fill 2046 79 2046 2051 79 2051 minecraft:stone','fill 2046 80 2046 2051 82 2051 minecraft:air','tp @s 2048.5 80 2047.5','setblock 2048 80 2049 tconstruct:part_builder','give @s tconstruct:pattern 1'):cmd(c)
 g.call('hotbar','9');g.call('wait','10');g.call('interact','2048','80','2049');g.call('wait','15')
 click(hot('tconstruct:pattern'));click(300);click(hot('tconstruct:earth_slime_crystal'));click(301)
 def arrow():return next((n for n in g.call('ui')['widgets'] if n.get('label')=='Arrow Head' and n.get('enabled')),None)
 node=wait_for('Earthslime Arrow Head option',arrow);g.call('click',node['id']);g.call('wait','15');g.call('screenshot',str(run/'earth-geode-builder.png'));click(322)
 part=g.call('menu').get('carried') or {};print('BUILDER RESULT',part,flush=True);assert part.get('item')=='tconstruct:arrow_head' and 'tconstruct:earthslime' in part.get('components',''),part
 empty=next(s['slot'] for s in g.call('inventory')['hotbar'] if not s.get('item'));click(empty);g.call('close')
 # Craft a block from four natural Ichor crystals; no recipe input is supplied.
 cmd('setblock 2048 80 2049 minecraft:crafting_table');g.call('hotbar','9');g.call('wait','10');g.call('interact','2048','80','2049');g.call('wait','10')
 source=hot('tconstruct:ichor_slime_crystal');click(source)
 for slot in (100,101,103,104):click(slot,'right')
 if g.call('menu')['carried']:click(source)
 preview={x['slot']:x.get('stack') for x in g.call('menu')['slots']};assert (preview.get(200) or {}).get('item')=='tconstruct:ichor_slime_crystal_block',preview
 click(200);empty=next(s['slot'] for s in g.call('inventory')['hotbar'] if not s.get('item'));click(empty);g.call('close');print('CRAFTED Ichor crystal block',flush=True)
 # The furnace and fuel are fixtures; Enderslime crystal is naturally mined.
 cmd('setblock 2048 80 2049 minecraft:blast_furnace');cmd('give @s minecraft:coal 1');g.call('hotbar','9');g.call('wait','10');g.call('interact','2048','80','2049');g.call('wait','10')
 source=hot('tconstruct:ender_slime_crystal');click(source);click(300,'right');click(source);click(hot('minecraft:coal'));click(301);g.call('wait','240',timeout=40)
 slots={x['slot']:x.get('stack') for x in g.call('menu')['slots']};print('BLAST SLOTS',slots,flush=True);assert (slots.get(302) or {}).get('item')=='tconstruct:ender_slime_ball',slots
 click(302);empty=next(s['slot'] for s in g.call('inventory')['hotbar'] if not s.get('item'));click(empty);g.call('close');g.call('save');g.stop();g.cleanup()
 g=Game(a,world,'Player',in_frame=a.in_frame,resume=True);g.ready();inv=g.call('inventory');print('RELOAD',inv['counts'],flush=True)
 assert any(s.get('item')=='tconstruct:arrow_head' and 'tconstruct:earthslime' in s.get('components','') for s in inv['hotbar'])
 assert inv['counts'].get('tconstruct:ichor_slime_crystal_block',0)>=1,inv['counts']
 assert inv['counts'].get('tconstruct:ender_slime_crystal',0)>=3 and inv['counts'].get('tconstruct:ender_slime_ball',0)>=1,inv['counts']
 g.stop();print('PASS natural Earth, Ichor and Ender geodes, LAN, builder and reload',flush=True)
finally:
 if guest:guest.cleanup()
 g.cleanup()

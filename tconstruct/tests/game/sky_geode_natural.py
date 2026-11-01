"""Find and harvest a naturally generated Sky Geode on a disposable world."""
import argparse, json
from pathlib import Path
from harness import Game

p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'): p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true')
p.add_argument('--seed',type=int,default=42)
a=p.parse_args();a.natural_world=True;a.world_seed=a.seed
run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
world=run/'Sky';world.mkdir(exist_ok=True)
assert not (world/'Player/saves/Probe').exists(), 'requires a new disposable Sky world'
g=Game(a,world,'Player',in_frame=a.in_frame,resume=(world/'Player').exists())
def cmd(s):g.call('command',s);g.call('wait','8')
def hot(i):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==i)
def click(i):g.call('click-slot',str(i));g.call('wait','8')
try:
 g.ready();cmd('gamerule minecraft:spawn_mobs false')
 candidates=[(x*64,z*64) for z in range(-2,3) for x in range(-2,3)]
 found=None
 for x,z in candidates:
  cmd(f'tp @s {x+0.5} 0 {z+0.5}');g.call('wait-ready','90',timeout=95);g.call('wait','15')
  info=g.call('find-block','tconstruct:sky_slime_crystal_cluster','64',timeout=90)
  print('SEARCH',a.seed,x,z,info,flush=True)
  if info['nearest']:
   found=info['nearest'][0]['pos'];break
 assert found is not None,'no naturally generated cluster found in sampled loaded chunks'
 x,y,z=found;print('NATURAL CLUSTER',a.seed,found,flush=True)
 g.call('save');g.call('wait','15');g.stop();g.cleanup()
 g=Game(a,world,'Player',in_frame=a.in_frame,resume=True);g.ready()
 info=g.call('block',x,y,z);print('RELOADED BLOCK',info,flush=True);assert info['block']=='tconstruct:sky_slime_crystal_cluster',info
 # The target is natural; clear only an adjacent approach to make the GUI-driven
 # mining fixture repeatable without changing the target or its loot.
 for c in (f'fill {x-1} {y-1} {z-2} {x+1} {y+2} {z-1} minecraft:air',f'tp @s {x+0.5} {y} {z-1.5}','clear @s','give @s minecraft:diamond_pickaxe 1','gamemode survival @s'):cmd(c)
 g.call('hotbar',str(hot('minecraft:diamond_pickaxe')+1));g.call('wait','10');g.call('screenshot',str(run/'sky-geode-natural.png'))
 g.call('break',x,y,z,'--timeout','30',timeout=35);g.call('wait','50')
 inv=g.call('inventory');print('HARVEST',inv['counts'],flush=True);assert inv['counts'].get('tconstruct:sky_slime_crystal',0)>=4,inv
 # Worktable and upgraded tool are fixtures; the crystal above was mined.
 for c in ('gamemode creative @s',f'fill {x-1} {y-1} {z-2} {x+2} {y+2} {z+2} minecraft:air',f'setblock {x} {y-1} {z-1} minecraft:stone',f'tp @s {x+0.5} {y} {z-1.5}',f'setblock {x} {y} {z} tconstruct:modifier_worktable','give @s minecraft:wet_sponge 1','give @s tconstruct:crossbow[minecraft:custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:string"],tic_max_damage:85,tic_upgrades:{"tconstruct:reinforced":4}},minecraft:max_damage=85] 1'):cmd(c)
 g.call('hotbar','9');g.call('wait','10');g.call('interact',x,y,z);g.call('wait','15')
 for item,slot in [('tconstruct:crossbow',300),('minecraft:wet_sponge',301),('tconstruct:sky_slime_crystal',302)]:click(hot(item));click(slot)
 g.call('click-gui','31','19');g.call('wait','15');g.call('screenshot',str(run/'sky-geode-worktable.png'))
 click(325);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:crossbow';click(0);g.call('close');g.call('wait','10')
 assert g.call('inventory')['counts'].get('tconstruct:modifier_crystal',0)>0
 g.call('save');g.stop();print('PASS natural Sky Geode cluster, reload, harvesting and Worktable',flush=True)
finally:g.cleanup()

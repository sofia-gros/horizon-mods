import argparse,sys
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve()), 'run-dir must be inside tconstruct/target';g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def mine():
 cmd('gamemode creative @s');cmd('tp @s 0.5 71 0.5');cmd('setblock 0 71 2 minecraft:obsidian');g.call('select','minecraft:diamond_pickaxe');cmd('gamemode survival @s');g.call('wait','20');g.call('look-at','block','0','71','2');g.call('wait','10')
 start=g.call('status')['ticks'];g.call('mouse','left','down')
 for _ in range(90):
  g.call('wait','4')
  if g.call('block','0','71','2')['block']=='minecraft:air': break
 else: raise AssertionError('block did not break')
 end=g.call('status')['ticks'];g.call('mouse','left','up');return end-start
try:
 g.ready()
 if g.call('status')['screen']=='dead': g.call('respawn');g.call('wait','20')
 for s in ('gamemode creative @s','clear @s','effect clear @s','effect give @s minecraft:instant_health 1 4 true','kill @e[type=minecraft:zombie]','kill @e[type=minecraft:skeleton]','kill @e[type=minecraft:spider]','kill @e[type=minecraft:creeper]','gamerule minecraft:spawn_mobs false','fill -2 70 -1 2 70 3 minecraft:stone','item replace entity @s armor.chest with minecraft:air','fill -2 71 -1 2 72 3 minecraft:air','give @s minecraft:diamond_pickaxe 1'):cmd(s)
 baseline=mine();print('BASE',baseline,flush=True)
 cmd('gamemode creative @s')
 cmd('item replace entity @s armor.chest with tconstruct:plate_chestplate[minecraft:max_damage=240,minecraft:custom_data={tic_max_damage:240,tic_materials:["tconstruct:iron","tconstruct:iron"],tic_upgrades:{"tconstruct:haste":90}}]')
 print('ARMOR',g.call('inventory')['armor'],flush=True)
 upgraded=mine();print('TICKS',baseline,upgraded,flush=True);assert upgraded < baseline,(baseline,upgraded)
 g.call('save');g.stop()
except Exception as e:
 print("ERR",getattr(e,"stdout",None),getattr(e,"stderr",None),flush=True);raise
finally:g.cleanup()

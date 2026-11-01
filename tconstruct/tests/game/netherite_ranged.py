"""Compare real Longbow/Crossbow arrow velocity after a Netherite transaction."""
import argparse,math,re
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==i)
def apply(kind, inputs):
 cmd('gamemode creative @s');cmd('tp @s 0.5 71 0.5');cmd('setblock 0 71 2 tconstruct:tinker_station')
 for item in inputs:cmd('give @s '+item+' 1')
 g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:'+kind));click(300)
 for idx,item in enumerate(inputs):click(hot(item));click(301+idx)
 click(325);out=g.call('menu')['carried'];assert out and out['item']=='tconstruct:'+kind,out
 click(0);g.call('close');g.call('wait','8')
def fire(kind):
 for c in ('gamemode creative @s','kill @e[type=minecraft:arrow]','tp @s 20.5 100 18.5','fill 17 100 17 24 110 40 minecraft:air','give @s minecraft:arrow 4','gamemode survival @s'):cmd(c)
 g.call('hotbar','1');g.call('look','0','-10');g.call('wait','20');g.call('mouse','right','down');g.call('wait','40');g.call('mouse','right','up')
 if kind=='crossbow':g.call('wait','5');g.call('mouse','right','down');g.call('wait','1');g.call('mouse','right','up')
 g.call('wait','2');data=g.call('command','data get entity @e[type=minecraft:arrow,sort=nearest,limit=1] Motion')
 values=re.findall(r'(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)d',str(data));assert len(values)==3,(kind,data)
 x,_,z=map(float,values);return math.hypot(x,z)
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for kind in ('longbow','crossbow'):
  cmd('gamemode creative @s');cmd('clear @s')
  parts='["tconstruct:wood","tconstruct:wood","tconstruct:wood","tconstruct:string"]' if kind=='longbow' else '["tconstruct:wood","tconstruct:wood","tconstruct:string"]'
  cmd('give @s tconstruct:'+kind+'[minecraft:custom_data={tic_materials:'+parts+',tic_max_damage:120,tic_draw_speed:1.0f,tic_velocity:1.0f,tic_accuracy:1.0f},minecraft:max_damage=120] 1')
  apply(kind,['minecraft:writable_book']);base=fire(kind)
  apply(kind,['minecraft:netherite_upgrade_smithing_template','minecraft:netherite_ingot']);upgraded=fire(kind)
  print('VELOCITY',kind,base,upgraded,flush=True);assert 1.06 < upgraded/base < 1.14,(kind,base,upgraded)
 g.call('save');g.stop();print('PASS actual Netherite ranged velocity',flush=True)
finally:g.cleanup()

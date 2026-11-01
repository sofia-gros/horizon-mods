"""Reject missing upgrade slots, ability slots and Unbreakable prerequisites."""
import argparse
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
def slots():return {x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
def run_case(tool,upgrades,ingredients):
 for c in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station',tool):cmd(c)
 for item in ingredients:cmd('give @s '+item+' 1')
 g.call('interact','0','71','2');g.call('wait','12');click(hot(upgrades));click(300)
 for i,item in enumerate(ingredients):
  name=item.split('[')[0];click(hot(name));click(301+i)
 g.call('wait','8');assert 325 not in slots(),slots();g.call('close');g.call('wait','8')
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 pick='tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85,tic_upgrades:{"tconstruct:sharpness":108}},minecraft:max_damage=85]'
 run_case('give @s '+pick+' 1','tconstruct:pickaxe',['minecraft:netherite_upgrade_smithing_template','minecraft:netherite_ingot'])
 pick='tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85,tic_upgrades:{"tconstruct:netherite":1,"tconstruct:reinforced":16,"tconstruct:writable":1,"tconstruct:harmonious":1}},minecraft:max_damage=85]'
 crystal='tconstruct:modifier_crystal[minecraft:custom_data={modifier:"tconstruct:unbreakable"}]'
 run_case('give @s '+pick+' 1','tconstruct:pickaxe',[crystal])
 armor='tconstruct:plate_helmet[minecraft:custom_data={tic_materials:["tconstruct:iron","tconstruct:iron"],tic_max_damage:120,tic_upgrades:{"tconstruct:netherite":1,"tconstruct:reinforced":20,"tconstruct:writable":1,"tconstruct:harmonious":1,"tconstruct:forecast":1,"tconstruct:recapitated":1}},minecraft:max_damage=120]'
 run_case('give @s '+armor+' 1','tconstruct:plate_helmet',[crystal])
 g.call('save');g.stop();print('PASS missing upgrade slot, Reinforced V and armor ability slot are rejected',flush=True)
finally:g.cleanup()

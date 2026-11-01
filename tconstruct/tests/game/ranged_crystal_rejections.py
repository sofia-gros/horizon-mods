import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve()), 'run-dir must be inside tconstruct/target';g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def slots():return {x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
def setup(table,amount):
 for s in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air',f'setblock 0 71 2 tconstruct:{table}',f'give @s tconstruct:crossbow[minecraft:custom_data={{tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:string"],tic_max_damage:85,tic_upgrades:{{"tconstruct:quick_charge":{amount}}}}},minecraft:max_damage=85] 1'):cmd(s)
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 setup('modifier_worktable',2);cmd('give @s minecraft:wet_sponge 1');cmd('give @s tconstruct:sky_slime_crystal 1');g.call('interact','0','71','2');g.call('wait','10')
 for item,slot in [('tconstruct:crossbow',300),('minecraft:wet_sponge',301),('tconstruct:sky_slime_crystal',302)]:click(hot(item));click(slot)
 g.call('click-gui','31','19');g.call('wait','8');assert 325 not in slots(),slots();assert all(i in slots() for i in (300,301,302)),slots();g.call('close')
 setup('tinker_station',15);cmd('give @s tconstruct:modifier_crystal[minecraft:custom_data={modifier:"tconstruct:trueshot"}] 1');g.call('interact','0','71','2');g.call('wait','10');click(hot('tconstruct:crossbow'));click(300);click(hot('tconstruct:modifier_crystal'));click(301);g.call('wait','8');assert 325 not in slots(),slots();assert all(i in slots() for i in (300,301)),slots();g.call('close')
 for c in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:crossbow[minecraft:custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:string"],tic_upgrades:{"tconstruct:trueshot":30},tic_max_damage:120},minecraft:max_damage=120] 1','give @s minecraft:target 1'):cmd(c)
 g.call('interact','0','71','2');g.call('wait','10');click(hot('tconstruct:crossbow'));click(300);click(hot('minecraft:target'));click(301);g.call('wait','8');assert 325 not in slots(),slots();assert all(i in slots() for i in (300,301)),slots();g.call('close');g.call('save');g.stop();print('PASS partial-only, exhausted slots and maximum-level rejection')
finally:g.cleanup()

"""Compare the same material repair with and without Emerald."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==i)
def repair(emerald):
 gem=',tic_upgrades:{"tconstruct:emerald":1}' if emerald else ''
 maximum=127 if emerald else 85
 tool=f'tconstruct:pickaxe[minecraft:damage=60,minecraft:max_damage={maximum},minecraft:custom_data={{tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:{maximum}{gem}}}]'
 for c in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s '+tool+' 1','give @s minecraft:flint 1'):cmd(c)
 g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:pickaxe'));click(300);click(hot('minecraft:flint'));click(301);click(325)
 out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:pickaxe',out
 amount=60-out.get('damage',0);print('repair',emerald,amount,out,flush=True);click(0);g.call('close');g.call('wait','5');return amount
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 base=repair(False);gem=repair(True);assert gem>base>0,(base,gem)
 g.call('save');g.stop();print('PASS Emerald repair amount',base,gem,flush=True)
finally:g.cleanup()

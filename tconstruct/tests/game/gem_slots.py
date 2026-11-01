"""Check ordinary gem input cannot exceed existing upgrade slots."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==i)
def slots():return {x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for c in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85,tic_upgrades:{"tconstruct:sharpness":108}},minecraft:max_damage=85] 1','give @s minecraft:diamond 1'):cmd(c)
 g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:pickaxe'));click(300);click(hot('minecraft:diamond'));click(301);g.call('wait','8');assert 325 not in slots() and 300 in slots() and 301 in slots(),slots()
 g.call('close');g.call('save');g.stop();print('PASS gem rejected without upgrade slots',flush=True)
finally:g.cleanup()

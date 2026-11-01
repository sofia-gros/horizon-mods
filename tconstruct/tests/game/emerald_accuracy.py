"""Measure actual lateral arrow motion from one bow before and after Emerald."""
import argparse,re
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==i)
def shot():
 for c in ('gamemode creative @s','kill @e[type=minecraft:arrow]','kill @e[tag=gem_arrow_target]','tp @s 20.5 100 18.5','fill 17 99 17 24 99 40 minecraft:stone','fill 17 100 17 24 110 40 minecraft:air','give @s minecraft:arrow 4','gamemode survival @s'):cmd(c)
 g.call('hotbar',str(hot('tconstruct:longbow')+1));g.call('look','0','-10');g.call('wait','15');g.call('mouse','right','down');g.call('wait','35');g.call('mouse','right','up');g.call('wait','2')
 data=g.call('command','data get entity @e[type=minecraft:arrow,sort=nearest,limit=1] Motion');vals=re.findall(r'(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)d',str(data));assert len(vals)==3,data;return float(vals[0])
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for c in ('gamemode creative @s','clear @s','give @s tconstruct:longbow[minecraft:custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:wood","tconstruct:string"],tic_max_damage:360,tic_draw_speed:1.0f,tic_velocity:1.0f,tic_accuracy:0.75f},minecraft:max_damage=360] 1'):cmd(c)
 base=[shot() for _ in range(5)]
 for c in ('gamemode creative @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:tinkers_anvil','give @s minecraft:emerald 1'):cmd(c)
 g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:longbow'));click(300);click(hot('minecraft:emerald'));click(301);click(325);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:longbow';click(0);g.call('close');g.call('wait','8')
 item=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:longbow');assert 'tic_accuracy:0.85f' in item.get('components',''),item
 modified=[shot() for _ in range(5)];b=sum(x*x for x in base);m=sum(x*x for x in modified);print('lateral shots',base,modified,'squared sums',b,m,flush=True);assert m<b,(base,modified)
 g.call('save');g.stop();print('PASS Emerald actual arrow spread',flush=True)
finally:g.cleanup()

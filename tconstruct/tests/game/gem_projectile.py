"""Compare actual crossbow arrow hits before and after Diamond."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==i)
def hit():
 for c in ('gamemode creative @s','kill @e[type=minecraft:arrow]','kill @e[tag=gem_arrow_target]','tp @s 20.5 100 18.5','fill 17 99 17 24 99 40 minecraft:stone','fill 17 100 17 24 110 40 minecraft:air','give @s minecraft:arrow 8','summon minecraft:cow 20.5 100 23.5 {NoAI:1b,Tags:["gem_arrow_target"]}','gamemode survival @s'):cmd(c)
 g.call('hotbar','1');targets=g.call('entities','15','minecraft:cow')['entities'];target=min(targets,key=lambda e:abs(e['pos'][2]-23.5));g.call('look-at','20.5','100.8','23.5');g.call('wait','20');g.call('mouse','right','down');g.call('wait','40');g.call('mouse','right','up');g.call('wait','5');g.call('mouse','right','down');g.call('wait','1');g.call('mouse','right','up');g.call('wait','15')
 after=g.call('entities','15','minecraft:cow')['entities'];mob=next((e for e in after if e['id']==target['id']),None);assert mob,(target,after)
 damage=target['health']-mob['health'];print('projectile hit',damage,flush=True);assert damage>0;return damage
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for c in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:crossbow[minecraft:custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:string"],tic_max_damage:120,tic_draw_speed:1.0f,tic_velocity:1.0f,tic_accuracy:1.0f},minecraft:max_damage=120] 1'):cmd(c)
 base=hit()
 for c in ('gamemode creative @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:tinker_station','give @s minecraft:diamond 1'):cmd(c)
 g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:crossbow'));click(300);click(hot('minecraft:diamond'));click(301);click(325);out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:crossbow',out
 g.call('screenshot',str(run/'gem-crossbow.png'));click(0);g.call('close');g.call('wait','10');gem=hit();assert gem>base,(base,gem)
 g.call('save');g.stop();print('PASS Diamond actual arrow damage',base,gem,flush=True)
finally:g.cleanup()

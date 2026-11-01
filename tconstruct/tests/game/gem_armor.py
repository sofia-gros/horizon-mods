"""Apply Diamond and exercise armor damage, wear and broken retention."""
import argparse,re
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==i)
def hurt():g.call('wait','30');cmd('damage @s 4 minecraft:mob_attack');return g.call('hurtlog')['players'][0]['hurts'][-1]
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for c in ('gamemode creative @s','effect clear @s','effect give @s minecraft:instant_health 1 4 true','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','item replace entity @s armor.head with minecraft:air','item replace entity @s armor.chest with minecraft:air','item replace entity @s armor.legs with minecraft:air','item replace entity @s armor.feet with minecraft:air','give @s tconstruct:plate_helmet[minecraft:custom_data={tic_materials:["tconstruct:iron","tconstruct:iron"],tic_max_damage:120},minecraft:max_damage=120] 1','give @s minecraft:diamond 1'):cmd(c)
 g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:plate_helmet'));click(300);click(hot('minecraft:diamond'));click(301);click(325)
 result=g.call('menu')['carried'] or {};assert result.get('item')=='tconstruct:plate_helmet',result
 g.call('screenshot',str(run/'gem-armor.png'));click(500);g.call('close');g.call('wait','8');armor=g.call('inventory')['armor'][0];print('armor',armor,flush=True)
 assert '"tconstruct:diamond":1' in armor.get('components','') and 'tconstruct:diamond_armor' in armor.get('components',''),armor
 cmd('gamemode survival @s');hit=hurt();print('hit',hit,flush=True);assert hit['armor']>0,hit
 cmd('gamemode creative @s');cmd('item modify entity @s armor.head {type:"minecraft:set_damage",function:"minecraft:set_damage",damage:0.001}');cmd('gamemode survival @s');hurt();helmet=g.call('inventory')['armor'][0];print('broken',helmet,flush=True)
 assert helmet.get('item')=='tconstruct:plate_helmet' and 'tic_broken:1b' in helmet.get('components',''),helmet
 g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready();helmet=g.call('inventory')['armor'][0];assert helmet.get('item')=='tconstruct:plate_helmet' and 'tic_broken:1b' in helmet.get('components',''),helmet
 g.stop();print('PASS Diamond armor damage and broken retention',flush=True)
finally:g.cleanup()

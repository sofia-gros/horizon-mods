"""Melt obsidian and emerald, cast four reinforcements and apply them."""
import argparse,subprocess
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','5')
def click(i,*args):g.call('click-slot',str(i),*args);g.call('wait','6')
def hot(i):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==i)
def select(i):g.call('hotbar',str(hot(i)+1));g.call('wait','10')
def empty():
 slot=next(x['slot'] for x in g.call('inventory')['hotbar'] if not x.get('item'));g.call('hotbar',str(slot+1));g.call('wait','10')
def count(i):return g.call('inventory')['counts'].get(i,0)
def smelt(item):
 cmd('tp @s 0.5 71 -1.5');empty();g.call('wait','30')
 try:g.call('interact','0','71','1')
 except subprocess.CalledProcessError as e:print('CONTROLLER',e.stdout,e.stderr,'BLOCK',g.call('block','0','71','1'),'STATUS',g.call('status'),flush=True);raise
 g.call('wait','15');click(hot(item));click(302);g.call('wait','600',timeout=50);slots={x['slot']:x.get('stack') for x in g.call('menu')['slots']};assert not slots.get(302),(item,slots);g.call('close')
def table(output,cast=None):
 cmd('tp @s 2.5 71 4.5')
 if cast:select(cast);g.call('interact','0','70','4');g.call('wait','5')
 empty();g.call('interact','0','71','4');g.call('wait','200',timeout=40);before=count(output);g.call('interact','0','70','4');g.call('wait','10');assert count(output)==before+1,(output,g.call('inventory'))
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for xyz in ((0,71,1),(1,71,2),(0,71,3),(0,71,4),(0,70,4)):cmd('setblock %d %d %d minecraft:air'%xyz)
 for c in ('gamemode creative @s','clear @s','tp @s 2.5 71 1.5','fill -2 71 -2 2 73 0 minecraft:air','fill -3 70 -3 3 70 5 minecraft:stone','setblock 0 70 2 tconstruct:seared_bricks','setblock 0 71 2 minecraft:air','setblock -1 71 2 tconstruct:seared_bricks','setblock 1 71 2 tconstruct:seared_fuel_tank','setblock 0 71 3 tconstruct:seared_drain[facing=south]','setblock 0 71 1 tconstruct:smeltery_controller[facing=north,lit=false]','setblock 0 71 4 tconstruct:seared_faucet[facing=south]','setblock 0 70 4 tconstruct:seared_table','give @s minecraft:lava_bucket 1','give @s minecraft:obsidian 1','give @s minecraft:emerald 1'):cmd(c)
 select('minecraft:lava_bucket');g.call('interact','1','71','2');g.call('wait','10')
 smelt('minecraft:obsidian')
 for _ in range(4):table('tconstruct:obsidian_pane')
 assert count('tconstruct:obsidian_pane')==4
 smelt('minecraft:emerald')
 for _ in range(4):table('tconstruct:emerald_reinforcement','tconstruct:obsidian_pane')
 assert count('tconstruct:emerald_reinforcement')==4 and count('tconstruct:obsidian_pane')==0
 g.call('screenshot',str(run/'reinforced-natural-cast.png'))
 for c in ('tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85},minecraft:max_damage=85] 1'):cmd(c)
 empty();g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:pickaxe'));click(300);source=hot('tconstruct:emerald_reinforcement');click(source);click(301,'right');click(source);click(325)
 result=g.call('menu')['carried'] or {};assert result.get('item')=='tconstruct:pickaxe',result;dest=next(x['slot'] for x in g.call('inventory')['hotbar'] if not x.get('item'));click(dest);g.call('close');g.call('wait','8');held=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:pickaxe');assert '"tconstruct:reinforced":1' in held.get('components',''),held
 g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:pickaxe'));click(300);click(hot('tconstruct:emerald_reinforcement'));click(301);click(325);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe';g.call('screenshot',str(run/'reinforced-station.png'));dest=next(x['slot'] for x in g.call('inventory')['hotbar'] if not x.get('item'));click(dest);g.call('close');g.call('wait','8')
 held=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:pickaxe');assert '"tconstruct:reinforced":4' in held.get('components',''),held
 assert count('tconstruct:emerald_reinforcement')==0;g.call('save');g.stop();g.cleanup()
 g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready();held=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:pickaxe');assert '"tconstruct:reinforced":4' in held.get('components',''),held
 g.stop();print('PASS natural Reinforced casting, partial/full application and reload',flush=True)
finally:g.cleanup()

import argparse,sys,re
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve()), 'run-dir must be inside tconstruct/target';g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def mine():
 cmd('gamemode creative @s');cmd('tp @s 0.5 71 0.5');cmd('setblock 0 71 2 minecraft:stone');cmd('gamemode survival @s');g.call('wait','20');return g.call('break','0','71','2','--timeout','30',timeout=35)['ticks']
def station():cmd('gamemode creative @s');cmd('setblock 0 71 2 tconstruct:tinker_station');g.call('interact','0','71','2');g.call('wait','12')
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for s in ('gamemode creative @s','clear @s','effect clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1','give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1','give @s tconstruct:tool_binding[minecraft:custom_data={Material:"tconstruct:wood"}] 1'):cmd(s)
 g.call('interact','0','71','2');g.call('wait','12')
 for i,item in enumerate(('tconstruct:pick_head','tconstruct:tool_handle','tconstruct:tool_binding')):click(hot(item));click(300+i)
 click(324);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe';click(0);g.call('close');g.call('hotbar','1');g.call('wait','5')
 baseline=mine();print('BASE',baseline,flush=True)
 cmd('gamemode creative @s');cmd('give @s minecraft:redstone 44');station();click(0);click(300);click(hot('minecraft:redstone'));click(301);click(325);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe';click(0);g.call('close')
 cmd('give @s minecraft:redstone_block 1');station();click(0);click(300);click(hot('minecraft:redstone_block'));click(301);click(325);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe';click(0);g.call('close');g.call('hotbar','1');g.call('wait','7')
 held=g.call('inventory')['held'];print('HASTE',held,flush=True);assert '"tconstruct:haste":45' in held.get('components',''),held;assert g.call('inventory')['counts'].get('minecraft:redstone',0)==8
 upgraded=mine();print('TICKS',baseline,upgraded,flush=True);assert upgraded<baseline,(baseline,upgraded)
 g.call('save');g.stop();g.cleanup()
 g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready();saved=g.call('inventory')['held']
 assert '"tconstruct:haste":45' in saved.get('components',''),saved
 print('PASS saved Haste',flush=True);g.stop()
finally:g.cleanup()

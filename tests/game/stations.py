"""Create parts, assemble and repair in one fresh single-player world."""
import argparse, tempfile, time
from pathlib import Path
from harness import Game

p=argparse.ArgumentParser()
p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True)
p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true')
p.add_argument('--output-parent',type=Path,default=Path('target'))
a=p.parse_args();a.output_parent.mkdir(parents=True,exist_ok=True)
run=Path(tempfile.mkdtemp(prefix='station-test-',dir=a.output_parent.resolve()));print('run',run,flush=True)
g=Game(a,run,'Player',in_frame=a.in_frame)
def click(slot):g.call('click-slot',str(slot));g.call('wait','3')
def hotbar(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def carried():return g.call('menu')['carried'] or {}
def cmd(value):return g.call('command',value)
def hover(slot, name):
 node = next(n for n in g.call('ui')['widgets'] if n['id'] == f'slot-{slot}')
 x,y,_,_ = node['bounds']
 g.call('scroll-at',str(x+8),str(y+8),'0');g.call('wait','5')
 g.call('screenshot',str(run/name))
try:
 g.ready()
 for command in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','give @s tconstruct:pattern 1','give @s minecraft:flint 2','give @s minecraft:oak_planks 2'):cmd(command)
 time.sleep(1);g.call('interact','0','71','2')
 g.call('screenshot',str(run/'part-builder-empty.png'))
 click(hotbar('tconstruct:pattern'));click(300);click(hotbar('minecraft:flint'));click(301)
 g.call('screenshot',str(run/'part-builder.png'))
 hover(301,'part-builder-hover.png')
 click(322);assert not carried()  # no selection means no output
 click(324);assert not carried()  # selecting is not result pickup
 g.call('scroll-at','0','0','0');g.call('wait','5');g.call('screenshot',str(run/'part-builder-selected.png'))
 hover(322,'part-builder-result-hover.png')
 click(322);assert carried().get('item')=='tconstruct:flint_pick_head'
 click(322);assert carried().get('count')==1
 click(3);g.call('wait','10');assert 'tconstruct:flint' in g.call('inventory')['hotbar'][3].get('components','')
 click(hotbar('minecraft:oak_planks'));click(301)
 click(325);assert not carried();click(322);assert carried().get('item')=='tconstruct:wood_handle';click(4)
 g.call('wait','10');assert 'tconstruct:wood' in g.call('inventory')['hotbar'][4].get('components','')
 click(326);assert not carried();click(322);assert carried().get('item')=='tconstruct:wood_binding';click(5)
 g.call('close');cmd('setblock 1 71 2 tconstruct:tinker_station');time.sleep(1);g.call('interact','1','71','2')
 g.call('screenshot',str(run/'tinker-station-empty.png'))
 for target,item in [(300,'tconstruct:flint_pick_head'),(301,'tconstruct:wood_handle'),(302,'tconstruct:wood_binding')]:click(hotbar(item));click(target)
 g.call('screenshot',str(run/'tinker-station.png'))
 hover(300,'tinker-station-hover.png')
 click(324);assert carried().get('item')=='tconstruct:flint_pickaxe';click(0);g.call('wait','10')
 assert 'tic_materials' in g.call('inventory')['held'].get('components','')
 g.call('close');g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
 assert 'tic_materials' in g.call('inventory')['held'].get('components','')
 cmd("item replace entity @s weapon.mainhand with tconstruct:flint_pickaxe[minecraft:damage=84,minecraft:custom_name='\"Blade\"',minecraft:custom_data={proof:17},minecraft:lore=['\"Durable story\"']]")
 cmd('give @s minecraft:flint 2');g.call('wait','10');held=g.call('inventory')['held'];print('before repair',held,flush=True)
 assert held.get('damage')==84 and 'Blade' in held.get('components','') and 'proof' in held.get('components','') and 'Durable story' in held.get('components','')
 g.call('interact','1','71','2');click(0);click(300);click(hotbar('minecraft:flint'));click(301)
 click(325);assert carried().get('item')=='tconstruct:flint_pickaxe' and carried().get('damage')==0;click(0)
 g.call('wait','10');hover(0,'lore-hover.png');held=g.call('inventory')['held'];print('after repair',held,flush=True);assert 'Blade' in held.get('components','') and 'proof' in held.get('components','') and 'Durable story' in held.get('components','')
 g.call('close');cmd('give @s minecraft:oak_planks 1');g.call('wait','10');g.call('interact','0','71','2')
 click(hotbar('minecraft:oak_planks'));click(301);click(325);click(322);click(8)
 assert 'tconstruct:wood' in g.call('inventory')['hotbar'][8].get('components','')
 g.call('close');g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
 assert 'tconstruct:wood' in g.call('inventory')['hotbar'][8].get('components','')
 held=g.call('inventory')['held'];assert held.get('damage',0)==0 and 'Blade' in held.get('components','') and 'proof' in held.get('components','') and 'Durable story' in held.get('components','')
 g.call('interact','0','71','2');g.call('wait','10');g.call('screenshot',str(run/'part-builder-reopened.png'))
 items={s['slot']:s['stack'] for s in g.call('menu')['slots']}
 assert items[300]['item']=='tconstruct:pattern'
 # Exercise Lore without custom_data as well as the combined case above.
 g.call('close');cmd("item replace entity @s weapon.mainhand with tconstruct:flint_pickaxe[minecraft:damage=84,minecraft:lore=['\"Lore only\"']]")
 cmd('give @s minecraft:flint 2');g.call('wait','15')
 assert 'Lore only' in g.call('inventory')['held'].get('components','')
 g.call('interact','1','71','2');click(0);click(300);click(hotbar('minecraft:flint'));click(301);click(325);click(0)
 assert 'Lore only' in g.call('inventory')['held'].get('components','')
 g.call('close');g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
 assert 'Lore only' in g.call('inventory')['held'].get('components','')
 g.stop();print('PASS station assembly, repair, data and save/reload',flush=True)
finally:g.cleanup()

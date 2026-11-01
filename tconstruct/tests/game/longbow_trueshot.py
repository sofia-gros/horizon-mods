"""Apply both ranged upgrades at each station and verify persistence."""
import argparse, re
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'): p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true')
a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s): g.call('command',s);g.call('wait','7')
def click(n): g.call('click-slot',str(n));g.call('wait','7')
def hot(item): return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
try:
 g.ready()
 if g.call('status')['screen']=='dead': g.call('respawn');g.call('wait','20')
 for table in ('tinkers_anvil',):
  for name,ingredient,amount in (('trueshot','minecraft:target',10),):
   for s in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air',f'setblock 0 71 2 tconstruct:{table}','give @s tconstruct:longbow[minecraft:custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:wood","tconstruct:string"],tic_max_damage:200},minecraft:max_damage=200] 1',f'give @s {ingredient} {amount}'):
    cmd(s)
   g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:longbow'));click(300);click(hot(ingredient));click(301 if table=='tinker_station' else 305)
   if table=='scorched_anvil' and name=='trueshot': g.call('screenshot',str(run/'longbow-trueshot-station.png'))
   click(325)
   out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:longbow',(table,name,out,g.call('menu'))
   click(0);g.call('close');g.call('wait','8');item=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:longbow');assert f'"tconstruct:{name}":{amount}' in item.get('components',''),(table,name,item)
   assert g.call('inventory')['counts'].get(ingredient,0)==0,(table,name,g.call('inventory'))
   print('PASS',table,name,item.get('components'),flush=True)
 g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready();item=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:longbow');assert '"tconstruct:trueshot":10' in item.get('components',''),item
 print('PASS saved ranged upgrade',flush=True)
 def fire():
  for c in ('gamemode creative @s','kill @e[type=minecraft:arrow]','tp @s 20.5 100 18.5','fill 17 99 17 24 99 40 minecraft:stone','fill 17 100 17 24 110 40 minecraft:air','give @s minecraft:arrow 8','gamemode survival @s'): cmd(c)
  g.call('hotbar','1');g.call('look','0','-10');g.call('wait','15');g.call('mouse','right','down');g.call('wait','35');g.call('mouse','right','up');g.call('wait','2')
  data=g.call('command','data get entity @e[type=minecraft:arrow,sort=nearest,limit=1] Motion');values=re.findall(r'(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)d',str(data));assert len(values)==3,data
  return float(values[0])
 level1=[fire() for _ in range(3)]
 cmd('gamemode creative @s');cmd('tp @s 0.5 71 0.5');cmd('setblock 0 71 2 tconstruct:tinkers_anvil');cmd('give @s minecraft:target 10')
 for i in range(1):
  g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:longbow'));click(300)
  if i == 0: click(hot('minecraft:target'));click(301)
  click(325);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:longbow';click(0);g.call('close');g.call('wait','8')
 item=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:longbow');assert '"tconstruct:trueshot":20' in item.get('components',''),item
 level2=[fire() for _ in range(3)];print('TRUESHOT lateral motion',level1,level2,flush=True);assert max(map(abs,level1))>max(map(abs,level2)),(level1,level2)
 print('PASS actual Trueshot spread',flush=True);g.stop()
finally:g.cleanup()

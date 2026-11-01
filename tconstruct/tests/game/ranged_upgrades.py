"""Apply both ranged upgrades at each station and verify persistence."""
import argparse
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
 for table in ('tinker_station','tinkers_anvil','scorched_anvil'):
  for name,ingredient,amount in (('quick_charge','minecraft:magma_cream',5),('trueshot','minecraft:target',10)):
   for s in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air',f'setblock 0 71 2 tconstruct:{table}','give @s tconstruct:crossbow[minecraft:custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:string"],tic_max_damage:200},minecraft:max_damage=200] 1',f'give @s {ingredient} {amount}'):
    cmd(s)
   g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:crossbow'));click(300);click(hot(ingredient));click(301 if table=='tinker_station' else 305)
   if table=='scorched_anvil' and name=='trueshot': g.call('screenshot',str(run/'ranged-upgrades-station.png'))
   click(325)
   out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:crossbow',(table,name,out,g.call('menu'))
   click(0);g.call('close');g.call('wait','8');item=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:crossbow');assert f'"tconstruct:{name}":{amount}' in item.get('components',''),(table,name,item)
   assert g.call('inventory')['counts'].get(ingredient,0)==0,(table,name,g.call('inventory'))
   print('PASS',table,name,item.get('components'),flush=True)
 g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready();item=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:crossbow');assert '"tconstruct:trueshot":10' in item.get('components',''),item
 print('PASS saved ranged upgrade',flush=True);g.stop()
finally:g.cleanup()

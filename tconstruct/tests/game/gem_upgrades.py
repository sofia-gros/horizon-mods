"""Exercise source gem recipes and duplicate rejection at all three stations."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'): p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(name):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==name)
def slots():return {x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for table in ('tinker_station','tinkers_anvil','scorched_anvil'):
  for gem in ('diamond','emerald'):
   for c in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air',f'setblock 0 71 2 tconstruct:{table}','give @s tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85},minecraft:max_damage=85] 1',f'give @s minecraft:{gem} 2'):cmd(c)
   g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:pickaxe'));click(300);click(hot('minecraft:'+gem));click(301 if table=='tinker_station' else 305)
   if gem=='diamond' and table=='scorched_anvil':g.call('screenshot',str(run/'gem-upgrade-station.png'))
   click(325);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe',(table,gem,slots())
   click(0);g.call('close');g.call('wait','8');inv=g.call('inventory');item=next(x for x in inv['hotbar'] if x.get('item')=='tconstruct:pickaxe');assert f'"tconstruct:{gem}":1' in item.get('components',''),item
   print('APPLIED',table,gem,item,flush=True)
   g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:pickaxe'));click(300);g.call('wait','5');assert 325 not in slots(),(table,gem,slots());g.call('close')
 g.call('save');g.stop();print('PASS gem transactions and duplicate refusal',flush=True)
finally:g.cleanup()

import argparse,sys
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for name in ('game','ctl','resources','run-dir'):p.add_argument('--'+name,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve()), 'run-dir must be inside tconstruct/target';g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for table in ('tinker_station','tinkers_anvil','scorched_anvil'):
  for command in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air',f'setblock 0 71 2 tconstruct:{table}', 'give @s tconstruct:crossbow[minecraft:custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:string"],tic_max_damage:85,tic_upgrades:{"tconstruct:trueshot":3}},minecraft:max_damage=85] 1','give @s tconstruct:modifier_crystal[minecraft:custom_data={modifier:"tconstruct:trueshot"}] 1'):
   cmd(command)
  g.call('interact','0','71','2');g.call('wait','12'); click(hot('tconstruct:crossbow'));click(300);click(hot('tconstruct:modifier_crystal'));click(301 if table=='tinker_station' else 305)
  g.call('wait','10'); click(325);out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:crossbow',(table,out)
  click(0);g.call('close');g.call('wait','8');inv=g.call('inventory');picked=next(x for x in inv['hotbar'] if x.get('item')=='tconstruct:crossbow');assert '"tconstruct:trueshot":13' in picked.get('components',''),(table,picked);assert inv['counts'].get('tconstruct:modifier_crystal',0)==0,inv
  print('PASS',table,picked.get('components'),flush=True)
 g.call('save');g.stop()
finally:g.cleanup()

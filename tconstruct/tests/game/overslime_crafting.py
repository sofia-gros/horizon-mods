"""Check bottle refill and remainders using supplied ingredients at a crafting table."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for name in ('game','ctl','resources','run-dir'): p.add_argument('--'+name,type=Path,required=True)
p.add_argument('--in-frame',action='store_true'); a=p.parse_args(); run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(value): g.call('command',value);g.call('wait','7')
def click(slot,*args): g.call('click-slot',str(slot),*args);g.call('wait','7')
def hot(item): return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for value in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air',
               'setblock 0 71 2 minecraft:crafting_table',
               'give @s tconstruct:pickaxe[minecraft:max_damage=85,minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85,tic_overslime:30,tic_overslime_capacity:50,tic_overslime_active:1b,tic_upgrades:{"tconstruct:overslime":1}}] 1',
               'give @s tconstruct:earth_slime_bottle 2'):cmd(value)
 g.call('interact','0','71','2');g.call('wait','12')
 click(hot('tconstruct:pickaxe'));click(100)
 click(hot('tconstruct:earth_slime_bottle'));click(101,'right');click(102)
 print('CRAFT MENU',g.call('menu'),flush=True)
 click(200);out=g.call('menu')['carried'];assert out and out['item']=='tconstruct:pickaxe',out
 click(0);g.call('close');g.call('wait','8')
 inv=g.call('inventory');print('CRAFT RESULT',inv,flush=True)
 item=next(x for x in inv['hotbar'] if x.get('item')=='tconstruct:pickaxe')
 assert 'tic_overslime:50' in item.get('components',''),item
 assert inv['counts'].get('minecraft:glass_bottle',0)==1 and inv['counts'].get('tconstruct:earth_slime_bottle',0)==1,inv
 for value in ('clear @s',
               'give @s tconstruct:pickaxe[minecraft:max_damage=85,minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85,tic_overslime:30,tic_overslime_capacity:50,tic_overslime_active:1b,tic_upgrades:{"tconstruct:overslime":1}}] 1',
               'give @s tconstruct:earth_slime_bottle 2'):cmd(value)
 g.call('interact','0','71','2');g.call('wait','12')
 click(hot('tconstruct:pickaxe'));click(100)
 click(hot('tconstruct:earth_slime_bottle'));click(101,'right');click(102)
 click(200,'shift');g.call('close');g.call('wait','8')
 inv=g.call('inventory');print('SHIFT CRAFT RESULT',inv,flush=True)
 item=next(x for x in inv['hotbar']+inv['main'] if x.get('item')=='tconstruct:pickaxe')
 assert 'tic_overslime:50' in item.get('components',''),item
 assert inv['counts'].get('minecraft:glass_bottle',0)==1 and inv['counts'].get('tconstruct:earth_slime_bottle',0)==1,inv
 g.call('save');g.stop();print('PASS bottle craft and remaining items',flush=True)
finally:g.cleanup()

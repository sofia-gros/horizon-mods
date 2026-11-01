"""Check Stringy repair using a supplied tool and String."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(v):g.call('command',v);g.call('wait','7')
def click(slot):g.call('click-slot',str(slot));g.call('wait','7')
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for v in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station',
           'give @s tconstruct:pickaxe[minecraft:damage=50,minecraft:max_damage=85,minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:flint","tconstruct:string"],tic_max_damage:85}] 1',
           'give @s minecraft:string 4'):cmd(v)
 g.call('interact','0','71','2');g.call('wait','12')
 for item,slot in [('tconstruct:pickaxe',300),('minecraft:string',301)]:click(hot(item));click(slot)
 shown={x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')};print('STRINGY OUTPUT',shown,flush=True)
 assert shown.get(325,{}).get('item')=='tconstruct:pickaxe' and shown[325].get('damage')==4,shown
 click(325);assert (g.call('menu')['carried'] or {}).get('damage')==4
 click(0);g.call('close');g.call('wait','8')
 for v in ('clear @s',
           'give @s tconstruct:pickaxe[minecraft:damage=50,minecraft:max_damage=85,minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:flint","tconstruct:leather"],tic_max_damage:85}] 1',
           'give @s minecraft:string 4'):cmd(v)
 g.call('interact','0','71','2');g.call('wait','12')
 for item,slot in [('tconstruct:pickaxe',300),('minecraft:string',301)]:click(hot(item));click(slot)
 without={x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
 assert 325 not in without and 300 in without and 301 in without,without
 g.call('close');g.call('save');g.stop();print('PASS Stringy source String repair and material rejection',flush=True)
finally:g.cleanup()

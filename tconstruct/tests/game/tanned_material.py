"""Build leather-binding tools and exercise primary wear before Overslime."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(v):g.call('command',v);g.call('wait','7')
def click(s):g.call('click-slot',str(s));g.call('wait','7')
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def build(shield):
 for v in ('gamemode creative @s','clear @s','kill @e[type=minecraft:zombie]','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station',
           'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
           'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
           'give @s tconstruct:tool_binding[minecraft:custom_data={Material:"tconstruct:leather"}] 1',
           'give @s minecraft:slime_ball 1'):cmd(v)
 g.call('interact','0','71','2');g.call('wait','12')
 for item,slot in [('tconstruct:pick_head',300),('tconstruct:tool_handle',301),('tconstruct:tool_binding',302)]:click(hot(item));click(slot)
 click(324);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe';click(0);g.call('close');g.call('wait','8')
 if shield:
  g.call('interact','0','71','2');g.call('wait','12')
  for item,slot in [('tconstruct:pickaxe',300),('minecraft:slime_ball',301)]:click(hot(item));click(slot)
  click(325);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe';click(0);g.call('close');g.call('wait','8')
 g.call('hotbar','1');g.call('wait','8')
def hit():
 cmd('gamemode survival @s');cmd('summon minecraft:zombie 1.5 71 0.5 {NoAI:1b,Tags:["tanned_test"]}');g.call('wait','30')
 target=g.call('entities','8','minecraft:zombie')['entities'][0]['id'];g.call('attack',str(target));g.call('wait','8')
 return g.call('inventory')['held']
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 build(False);tool=hit();print('TANNED NORMAL',tool,flush=True);assert tool.get('damage')==1,tool
 build(True);before=g.call('inventory')['held'];assert 'tic_overslime:20' in before.get('components',''),before
 after=hit();print('TANNED SHIELD',after,flush=True);assert after.get('damage',0)==0 and 'tic_overslime:19' in after.get('components',''),after
 g.call('screenshot',str(run/'tanned-material.png'));g.call('save');g.stop();g.cleanup()
 g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready();saved=g.call('inventory')['held']
 assert 'tconstruct:leather' in saved.get('components','') and 'tic_overslime:19' in saved.get('components',''),saved
 g.stop();print('PASS built leather Tanned wear before Overslime and reload',flush=True)
finally:g.cleanup()

"""Check source Enhanced slot use and material-swap validation."""
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
def slots():return {x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
def setup(amount):
 for v in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station',
           'give @s tconstruct:pickaxe[minecraft:max_damage=100,minecraft:custom_data={tic_materials:["tconstruct:rose_gold","tconstruct:flint","tconstruct:wood"],tic_max_damage:100,tic_upgrades:{"tconstruct:reinforced":'+str(amount)+',"tconstruct:diamond":1}}] 1',
           'give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1'):cmd(v)
 g.call('interact','0','71','2');g.call('wait','12')
 for item,slot in [('tconstruct:pickaxe',300),('tconstruct:pick_head',301)]:click(hot(item));click(slot)
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for v in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station',
           'give @s tconstruct:pickaxe[minecraft:max_damage=100,minecraft:custom_data={tic_materials:["tconstruct:rose_gold","tconstruct:flint","tconstruct:wood"],tic_max_damage:100,tic_upgrades:{"tconstruct:reinforced":12}}] 1',
           'give @s minecraft:diamond 1'):cmd(v)
 g.call('interact','0','71','2');g.call('wait','12')
 for item,slot in [('tconstruct:pickaxe',300),('minecraft:diamond',301)]:click(hot(item));click(slot)
 assert slots().get(325,{}).get('item')=='tconstruct:pickaxe',slots()
 click(325);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe';click(0);g.call('close');g.call('wait','8')
 enhanced=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:pickaxe')
 assert '"tconstruct:diamond":1' in enhanced.get('components',''),enhanced
 cmd('give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1')
 g.call('interact','0','71','2');g.call('wait','12')
 for item,slot in [('tconstruct:pickaxe',300),('tconstruct:pick_head',301)]:click(hot(item));click(slot)
 rejected=slots();print('REJECTED SWAP',rejected,flush=True)
 assert 325 not in rejected and 300 in rejected and 301 in rejected,rejected
 g.call('screenshot',str(run/'enhanced-swap-rejection.png'));g.call('close');g.call('wait','8')
 setup(8);accepted=slots();print('ACCEPTED SWAP',accepted,flush=True)
 assert accepted.get(325,{}).get('item')=='tconstruct:pickaxe',accepted
 click(325);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe'
 click(0);g.call('close');g.call('wait','8')
 item=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:pickaxe')
 assert 'tconstruct:flint' in item.get('components','') and 'tconstruct:rose_gold' not in item.get('components',''),item
 g.call('save');g.stop();print('PASS Enhanced replacement rejection and valid swap',flush=True)
finally:g.cleanup()

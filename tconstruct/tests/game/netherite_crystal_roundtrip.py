"""Extract and reapply a Netherite modifier Crystal."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==i)
def slots():return {x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
def pick():return next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:pickaxe')
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 tool='tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:102,"tconstruct:netherite":1b,tic_upgrades:{"tconstruct:netherite":1}},minecraft:max_damage=102]'
 for c in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:modifier_worktable','give @s '+tool+' 1','give @s minecraft:wet_sponge 1','give @s tconstruct:sky_slime_crystal 1'):cmd(c)
 g.call('interact','0','71','2');g.call('wait','12')
 for item,slot in [('tconstruct:pickaxe',300),('minecraft:wet_sponge',301),('tconstruct:sky_slime_crystal',302)]:click(hot(item));click(slot)
 g.call('click-gui','31','19');g.call('wait','12');assert 325 in slots(),slots();click(325)
 out=g.call('menu')['carried'];assert out and out['item']=='tconstruct:pickaxe',out
 click(0);g.call('close');g.call('wait','8');assert '"tconstruct:netherite":1' not in pick().get('components',''),pick()
 inv=g.call('inventory');assert any(x.get('item')=='tconstruct:modifier_crystal' and 'tconstruct:netherite' in x.get('components','') for x in inv['hotbar']),inv
 cmd('setblock 0 71 2 tconstruct:tinker_station');g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:pickaxe'));click(300);click(hot('tconstruct:modifier_crystal'));click(301)
 assert 325 in slots(),slots();click(325);click(0);g.call('close');g.call('wait','8');assert '"tconstruct:netherite":1' in pick().get('components',''),pick()
 g.call('save');g.stop();print('PASS Netherite Crystal extraction and reapplication',flush=True)
finally:g.cleanup()

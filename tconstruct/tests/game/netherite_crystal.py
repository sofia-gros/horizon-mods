"""Check Unbreakable's Crystal, prerequisites and Worktable selection."""
import argparse
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
for name in ('game','ctl','resources','run-dir'): p.add_argument('--'+name,type=Path,required=True)
p.add_argument('--in-frame',action='store_true')
a=p.parse_args();run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(slot):g.call('click-slot',str(slot));g.call('wait','7')
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def slots():return {x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for command in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:modifier_worktable',
  'give @s tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:100,tic_upgrades:{"tconstruct:netherite":1,"tconstruct:reinforced":20,"tconstruct:unbreakable":1,"tconstruct:writable":1,"tconstruct:harmonious":1,"tconstruct:forecast":1}},minecraft:max_damage=100] 1',
  'give @s minecraft:wet_sponge 1','give @s tconstruct:sky_slime_crystal 1'):cmd(command)
 g.call('interact','0','71','2');g.call('wait','12')
 for item,slot in [('tconstruct:pickaxe',300),('minecraft:wet_sponge',301),('tconstruct:sky_slime_crystal',302)]:click(hot(item));click(slot)
 # Reinforced, Netherite and Unbreakable are in that order. Removing either
 # prerequisite while Unbreakable remains must leave the result empty.
 for x in ('31','49'):
  g.call('click-gui',x,'19');g.call('wait','8');assert 325 not in slots(),slots()
 g.call('click-gui','67','19');g.call('wait','12');assert 325 in slots(),slots()
 g.call('screenshot',str(run/'netherite-crystal-icons.png'))
 click(325);result=g.call('menu')['carried'];assert result and result['item']=='tconstruct:pickaxe',result
 click(0);g.call('close');g.call('wait','8')
 loaded=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:pickaxe')
 assert '"tconstruct:unbreakable"' not in loaded.get('components',''),loaded
 inv=g.call('inventory');assert any(x.get('item')=='tconstruct:modifier_crystal' and 'tconstruct:unbreakable' in x.get('components','') for x in inv['hotbar']),inv
 cmd('setblock 0 71 2 tconstruct:tinker_station');g.call('interact','0','71','2');g.call('wait','12')
 click(hot('tconstruct:pickaxe'));click(300);click(hot('tconstruct:modifier_crystal'));click(301)
 assert 325 in slots(),slots();click(325);result=g.call('menu')['carried']
 assert result and result.get('item')=='tconstruct:pickaxe',result
 click(0);g.call('close');g.call('wait','8')
 loaded=next(x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:pickaxe')
 assert '"tconstruct:unbreakable":1' in loaded.get('components',''),loaded
 g.call('save');g.stop();print('PASS Unbreakable Crystal and dependency protection',flush=True)
finally:g.cleanup()

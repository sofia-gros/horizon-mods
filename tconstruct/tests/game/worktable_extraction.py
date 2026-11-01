import argparse,sys,re
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for name in ('game','ctl','resources','run-dir'):p.add_argument('--'+name,type=Path,required=True)
p.add_argument('--in-frame',action='store_true'); a=p.parse_args(); run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve()), 'run-dir must be inside tconstruct/target'
g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):
 r=g.call('command',s);g.call('wait','8');return r
def click(n):g.call('click-slot',str(n));g.call('wait','8')
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def slots():return {s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for command in ('gamemode creative @s','time set day','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:modifier_worktable','give @s tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85,tic_upgrades:{"tconstruct:sharpness":54}},minecraft:max_damage=85] 1','give @s minecraft:wet_sponge 1','give @s tconstruct:sky_slime_crystal 1'):
  cmd(command)
 print('BLOCK',g.call('block','0','71','2'),flush=True); print('INTERACT',g.call('interact','0','71','2'),flush=True);g.call('wait','15');print('UI',g.call('ui'),flush=True)
 for item,slot in [('tconstruct:pickaxe',300),('minecraft:wet_sponge',301),('tconstruct:sky_slime_crystal',302)]:click(hot(item));click(slot)
 g.call('click-gui','31','19');g.call('wait','15')
 print('SLOTS',slots(),flush=True)
 g.call('screenshot',str(run/'modifier-worktable-extraction.png'))
 click(325);out=g.call('menu')['carried'];print('RESULT',out,flush=True)
 assert out and out['item']=='tconstruct:pickaxe',out
 click(0);g.call('close');g.call('wait','10');inv=g.call('inventory');print('INVENTORY',inv,flush=True)
 assert any(x.get('item')=='tconstruct:modifier_crystal' for x in inv['hotbar']),inv
 assert any(x.get('item')=='minecraft:sponge' for x in inv['hotbar']),inv
 assert any(x.get('item')=='tconstruct:pickaxe' and '"tconstruct:sharpness":18' in x.get('components','') for x in inv['hotbar']),inv
 g.call('save');g.stop()
finally:g.cleanup()

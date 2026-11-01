"""Check a Netherite-tagged tool survives a representative lava drop."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for command in ('gamemode creative @s','clear @s','tp @s 0.5 73 0.5','fill -8 70 -8 8 70 8 minecraft:lava',
  'give @s tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:100,"tconstruct:netherite":1b,tic_upgrades:{"tconstruct:netherite":1}},minecraft:max_damage=100] 1'):
  cmd(command)
 g.call('hotbar','1');g.call('look','0','-35');g.call('wait','5');g.call('key','q','tap');g.call('wait','5')
 cmd('tp @s 0.5 80 0.5');g.call('wait','120')
 entities=g.call('entities','25','minecraft:item')['entities']
 items=[e for e in entities if (e.get('item') or {}).get('item')=='tconstruct:pickaxe']
 print('DROPPED_ITEMS',items,flush=True);assert items,entities
 g.call('screenshot',str(run/'netherite-lava-drop.png'))
 g.call('save');g.stop();print('PASS Netherite-tagged dropped tool remains after lava exposure',flush=True)
finally:g.cleanup()

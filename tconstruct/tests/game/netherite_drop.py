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
 for command in ('gamemode creative @s','clear @s','tp @s 0.5 73 0.5','fill -8 68 -8 8 68 8 minecraft:stone','fill -8 69 -8 8 69 8 minecraft:lava','fill -8 70 -8 8 72 8 minecraft:air',
  'give @s tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:100,"tconstruct:netherite":1b,tic_upgrades:{"tconstruct:netherite":1}},minecraft:max_damage=100] 1'):
  cmd(command)
 g.call('hotbar','1');g.call('look','0','-35');g.call('wait','5')
 print('DROP_BEFORE',g.call('inventory')['held'],flush=True)
 g.call('key','q','tap');g.call('wait','5');print('DROP_AFTER',g.call('inventory')['held'],g.call('entities','25','minecraft:item'),flush=True)
 cmd('tp @s 12.5 73 0.5')
 touched = None
 for _ in range(30):
  g.call('wait','5')
  entities=g.call('entities','25','minecraft:item')['entities']
  items=[e for e in entities if (e.get('item') or {}).get('item')=='tconstruct:pickaxe']
  if _ < 5: print('DROP_TICK',_,items,flush=True)
  for item in items:
   x,y,z=item['pos']
   if -8 <= x < 9 and -8 <= z < 9 and 69.0 <= y + 0.1 < 70.0:
    touched=item['id'];break
  if touched is not None:break
 assert touched is not None, ('no observed lava contact',items)
 g.call('wait','120')
 entities=g.call('entities','25','minecraft:item')['entities']
 items=[e for e in entities if e.get('id')==touched]
 print('DROPPED_ITEMS',items,flush=True);assert items,entities
 g.call('screenshot',str(run/'netherite-lava-drop.png'))
 cmd('fill -8 69 -8 8 70 8 minecraft:stone')
 g.call('save');g.stop();print('PASS Netherite-tagged dropped tool remains after lava exposure',flush=True)
finally:
 if g.proc.poll() is None:
  try:cmd('fill -8 69 -8 8 70 8 minecraft:stone')
  except Exception:pass
 g.cleanup()

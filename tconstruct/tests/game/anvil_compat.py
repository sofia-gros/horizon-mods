import argparse,sys
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources','run_dir'):p.add_argument('--'+k.replace('_','-'),type=Path,required=True)
a=p.parse_args()
if not a.run_dir.resolve().is_relative_to((Path(__file__).resolve().parents[2] / 'target').resolve()):
 raise SystemExit('run-dir must be inside tconstruct/target')
g=Game(a,a.run_dir,'Player',resume=True)
def cmd(s):g.call('command',s);g.call('wait','6')
def click(s,*flags):g.call('click-slot',str(s),*flags);g.call('wait','5')
def hot(value):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')=='tconstruct:fake_storage_block' and value in s.get('components',''))
def result():return next((s.get('stack') for s in g.call('menu')['slots'] if s['slot']==200),None)
try:
 g.ready()
 for c in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:crafting_table',
           'give @s tconstruct:fake_storage_block[minecraft:custom_data={Material:"tconstruct:bronze"}] 2',
           'give @s tconstruct:fake_storage_block[minecraft:custom_data={Material:"tconstruct:invar"}] 1',
           'give @s tconstruct:seared_bricks 4'):cmd(c)
 g.call('interact','0','71','2');g.call('wait','10')
 a1=hot('tconstruct:bronze');a2=hot('tconstruct:invar');click(a1);click(100,'right');click(101);click(a2);click(102)
 b=next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')=='tconstruct:seared_bricks');click(b)
 for slot in (104,106,107,108):click(slot,'right')
 if g.call('menu')['carried']:click(b)
 assert not result(),('mixed material produced output',result())
 click(102);click(a2)
 cmd('give @s tconstruct:fake_storage_block[minecraft:custom_data={Material:"tconstruct:bronze"}] 1')
 click(hot('tconstruct:bronze'));click(102);g.call('wait','10')
 out=result() or {};assert out.get('item')=='tconstruct:tinkers_anvil',out
 click(200);out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:tinkers_anvil',out
 print('PASS matching compatibility alloy; mixed material rejected',flush=True)
 g.call('close');g.stop()
finally:g.cleanup()

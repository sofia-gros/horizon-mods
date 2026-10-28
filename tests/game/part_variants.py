"""Check the six currently supported material/part combinations in a fresh world."""
import argparse, tempfile, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True)
p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true')
p.add_argument('--output-parent',type=Path,default=Path('target'))
a=p.parse_args();a.output_parent.mkdir(parents=True,exist_ok=True)
run=Path(tempfile.mkdtemp(prefix='part-variants-',dir=a.output_parent.resolve()));print('run',run,flush=True)
g=Game(a,run,'Player',in_frame=a.in_frame)
def click(slot):g.call('click-slot',str(slot));g.call('wait','3')
def hotbar(item):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
try:
 g.ready()
 for cmd in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','give @s tconstruct:pattern 6','give @s minecraft:flint 4','give @s minecraft:oak_planks 4'):g.call('command',cmd)
 time.sleep(1);g.call('interact','0','71','2');click(hotbar('tconstruct:pattern'));click(300)
 for input_id,material,prefix,models,start in [('minecraft:flint','tconstruct:flint','Flint',['flint_pick_head','flint_handle','flint_binding'],3),('minecraft:oak_planks','tconstruct:wood','Wooden',['wood_pick_head','wood_handle','wood_binding'],6)]:
  click(hotbar(input_id));click(301)
  for i,(button,item) in enumerate([(324,'tconstruct:pick_head'),(325,'tconstruct:tool_handle'),(326,'tconstruct:tool_binding')]):
   click(button);click(322)
   assert (g.call('menu')['carried'] or {}).get('item') == item
   click(start+i);g.call('wait','5')
   part=g.call('inventory')['hotbar'][start+i]
   assert material in part.get('components','') and models[i] in part.get('components','') and prefix in part.get('components',''),part
  g.call('screenshot',str(run/(material.split(':')[1]+'-parts.png')))
 g.call('close');g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
 for i,material in [(3,'tconstruct:flint'),(4,'tconstruct:flint'),(5,'tconstruct:flint'),(6,'tconstruct:wood'),(7,'tconstruct:wood'),(8,'tconstruct:wood')]:
  assert material in g.call('inventory')['hotbar'][i].get('components','')
 g.stop();print('PASS six parts and save/reload',flush=True)
finally:g.cleanup()

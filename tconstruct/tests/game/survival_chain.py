"""Use a single disposable world for the source early Smeltery progression.

Vanilla raw inputs, furnace, crafting table, fuel and a pickaxe are fixtures.
All TConstruct outputs used later must come from prior crafting or casting.
"""
import argparse,sys,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--phase',required=True,choices=['base','grout','furnace','devices','controller','alloy','lan','reload'])
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
g=Game(a,run,'Player',in_frame=a.in_frame,resume=(run/'Player').exists());guest=None

def cmd(s):
 result=g.call('command',s);g.call('wait','5');return result
def click(slot,*flags):g.call('click-slot',str(slot),*flags);g.call('wait','4')
def inventory():return g.call('inventory')
def count(item):return inventory()['counts'].get(item,0)
def slots(item):return [s for s in inventory()['hotbar']+inventory()['main'] if s.get('item')==item]
def slot(item):return slots(item)[0]['slot']
def hot(item):return next(s['slot'] for s in inventory()['hotbar'] if s.get('item')==item)
def empty():return next(s['slot'] for s in inventory()['hotbar'] if not s.get('item'))
def open_table():
 cmd('tp @s 0.5 71 0.5');g.call('hotbar',str(empty()+1));g.call('wait','5');g.call('interact','0','71','2');g.call('wait','10')
def craft(ingredients,result,amount=1,many=False):
 open_table()
 for item,positions in ingredients:
  source=slot(item);click(source)
  if many and len(positions)==1:click(positions[0])
  else:
   for dest in positions:click(dest,'right')
   if g.call('menu')['carried']:click(source)
 entries={x['slot']:x.get('stack') for x in g.call('menu')['slots']};preview=entries.get(200) or {}
 assert preview.get('item')==result,(result,preview,entries)
 if many:click(200,'shift')
 else:
  click(200);output=g.call('menu')['carried'];assert output and output.get('item')==result and output.get('count')==amount,output
  click(empty())
 g.call('close');g.call('wait','8');print('CRAFT',result,count(result),flush=True)
def widget(label):return next((w for w in g.call('ui')['widgets'] if w.get('label')==label and w.get('enabled')),None)
def wait_widget(label):
 for _ in range(120):
  w=widget(label)
  if w:return w
  time.sleep(.2)
 raise AssertionError('missing widget '+label)
def sprint(ticks):
 print('SPRINT',ticks,flush=True);g.call('command',f'tick sprint {ticks}');
 # Game control stays available between sprint batches. Poll for completion;
 # clock queries do not change the generated product.
 for _ in range(240):
  time.sleep(.5)
  msg=g.call('command','tick query',timeout=50)
  if 'sprinting' not in str(msg).lower():break
 else:raise AssertionError('tick sprint did not finish')
 g.call('wait','10');print('SPRINT FINISHED',flush=True)
def checkpoint():g.call('save');g.stop();print('PASS PHASE',a.phase,flush=True)

try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 if a.phase=='base':
  for c in ('gamemode creative @s','gamerule minecraft:spawn_mobs false','fill -5 70 -5 8 70 8 minecraft:stone','fill -5 71 -5 8 75 8 minecraft:air','tp @s 0.5 71 0.5','clear @s','setblock 0 71 2 minecraft:crafting_table','setblock 4 71 2 minecraft:furnace','give @s minecraft:oak_planks 12','give @s minecraft:stick 2','give @s minecraft:oak_log 1','give @s minecraft:flint 1'):cmd(c)
  craft([('minecraft:oak_planks',[100,104]),('minecraft:stick',[101,103])],'tconstruct:pattern',6)
  craft([('tconstruct:pattern',[100,101]),('minecraft:oak_planks',[103,104])],'tconstruct:part_builder')
  craft([('tconstruct:pattern',[100,101,102]),('minecraft:oak_planks',[103,105,106,108])],'tconstruct:tinker_station')
  cmd('tp @s 1.5 71 0.5');g.call('place','1','71','2','--item','tconstruct:part_builder');g.call('wait','10')
  g.call('hotbar',str(empty()+1));g.call('wait','5');g.call('interact','1','71','2');g.call('wait','15')
  click(slot('tconstruct:pattern'));click(300);click(slot('minecraft:oak_log'));click(301)
  g.call('click',wait_widget('Tool Handle')['id']);g.call('wait','10');click(322);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:tool_handle';click(empty())
  # The remaining planks from the log provide a binding; a vanilla flint makes the head.
  click(slot('minecraft:oak_planks'));click(301);g.call('click',wait_widget('Tool Binding')['id']);g.call('wait','10');click(322);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:tool_binding';click(empty())
  # Return any unconsumed material before changing the input.
  st={x['slot']:x.get('stack') for x in g.call('menu')['slots']};
  if st.get(301):click(301);click(empty())
  click(slot('minecraft:flint'));click(301);g.call('click',wait_widget('Pick Head')['id']);g.call('wait','10');click(322);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pick_head';click(empty());g.call('close')
  cmd('tp @s 2.5 71 0.5');g.call('place','2','71','2','--item','tconstruct:tinker_station');g.call('wait','10');g.call('hotbar',str(empty()+1));g.call('interact','2','71','2');g.call('wait','15')
  g.call('click',wait_widget('Pickaxe')['id']);g.call('wait','10')
  for item,target in [('tconstruct:pick_head',300),('tconstruct:tool_handle',301),('tconstruct:tool_binding',302)]:click(slot(item));click(target)
  g.call('wait','10');click(324);out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:pickaxe',out;click(empty());g.call('close');assert count('tconstruct:pickaxe')==1
  g.call('screenshot',str(run/'survival-station.png'));checkpoint()
 elif a.phase=='grout':
  for c in ('give @s minecraft:clay_ball 42','give @s minecraft:sand 42','give @s minecraft:gravel 42'):cmd(c)
  craft([('minecraft:clay_ball',[100]),('minecraft:sand',[101]),('minecraft:gravel',[102])],'tconstruct:grout',2,many=True)
  assert count('tconstruct:grout')==84,inventory();checkpoint()
 elif a.phase=='furnace':
  cmd('give @s minecraft:coal 12');cmd('tp @s 3.5 71 0.5');g.call('hotbar',str(empty()+1));g.call('interact','4','71','2');g.call('wait','10')
  largest=max(slots('tconstruct:grout'),key=lambda s:s.get('count',0));click(largest['slot']);click(300)
  coal=slot('minecraft:coal');click(coal)
  for _ in range(8):click(301,'right')
  if g.call('menu')['carried']:click(coal)
  sprint(13000)
  state={x['slot']:x.get('stack') for x in g.call('menu')['slots']};print('FURNACE FIRST',state,flush=True);assert (state.get(302) or {}).get('item')=='tconstruct:seared_brick' and state[302].get('count')==64,state
  click(302,'shift');click(slot('tconstruct:grout'));click(300);sprint(4300)
  state={x['slot']:x.get('stack') for x in g.call('menu')['slots']};print('FURNACE SECOND',state,flush=True);assert (state.get(302) or {}).get('item')=='tconstruct:seared_brick' and state[302].get('count')==20,state
  click(302,'shift');g.call('close');assert count('tconstruct:seared_brick')==84,inventory();g.call('screenshot',str(run/'survival-bricks.png'));checkpoint()
 else:raise RuntimeError('phase not yet implemented')
finally:
 if guest:guest.cleanup()
 g.cleanup()

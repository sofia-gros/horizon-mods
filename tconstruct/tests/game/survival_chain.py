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
def slot(item):return max(slots(item),key=lambda s:s.get('count',0))['slot']
def hot(item):return next(s['slot'] for s in inventory()['hotbar'] if s.get('item')==item)
def empty_any():return next(s['slot'] for s in inventory()['hotbar']+inventory()['main'] if not s.get('item'))
def empty():
 inv=inventory();free=next((s['slot'] for s in inv['hotbar'] if not s.get('item')),None)
 if free is not None:return free
 target=next(s['slot'] for s in inv['main'] if not s.get('item'))
 g.call('open-inventory');g.call('wait','8');click(8);click(target);g.call('close');return 8
def promote(item):
 inv=inventory();found=next((s['slot'] for s in inv['hotbar'] if s.get('item')==item),None)
 if found is not None:return found
 source=next(s['slot'] for s in inv['main'] if s.get('item')==item);target=empty()
 g.call('open-inventory');g.call('wait','8');click(source);click(target);g.call('close');return target
def place(item,x,y,z):
 promote(item);before=count(item);g.call('place',str(x),str(y),str(z),'--item',item);g.call('wait','10')
 assert g.call('block',str(x),str(y),str(z)).get('block')==item,(item,x,y,z)
 assert count(item)==before-1,(item,before,count(item))
def empty_hand():g.call('hotbar',str(empty()+1));g.call('wait','8')
def merge_stacks(item):
 found=slots(item)
 if len(found)<2:return
 found=sorted(found,key=lambda s:s.get('count',0));g.call('open-inventory');g.call('wait','8');click(found[0]['slot']);click(found[-1]['slot'])
 if g.call('menu')['carried']:click(found[0]['slot'])
 g.call('close')
TABLE=(0,71,2)
def open_table():
 x,y,z=TABLE;cmd(f'tp @s {x+0.5} {y} {z-1.5}');g.call('hotbar',str(empty()+1));g.call('wait','5');g.call('interact',str(x),str(y),str(z));g.call('wait','10')
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
  click(empty_any())
 g.call('close');g.call('wait','8');print('CRAFT',result,count(result),flush=True)
def widget(label):return next((w for w in g.call('ui')['widgets'] if w.get('label')==label and w.get('enabled')),None)
def wait_widget(label):
 for _ in range(120):
  w=widget(label)
  if w:return w
  time.sleep(.2)
 raise AssertionError('missing widget '+label)
def sprint(ticks):
 start=g.call('state')['ticks'];print('SPRINT',ticks,'FROM',start,flush=True);g.call('command',f'tick sprint {ticks}')
 # Poll the actual simulation counter rather than guessing from wall time.
 for _ in range(360):
  time.sleep(.5)
  if g.call('state',timeout=50)['ticks'] >= start+ticks:break
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
  click(302,'shift');click(slot('tconstruct:grout'));click(300);click(slot('minecraft:coal'));click(301);sprint(4300)
  state={x['slot']:x.get('stack') for x in g.call('menu')['slots']};print('FURNACE SECOND',state,flush=True);assert (state.get(302) or {}).get('item')=='tconstruct:seared_brick' and state[302].get('count')==20,state
  click(302,'shift');g.call('close');assert count('tconstruct:seared_brick')==84,inventory();g.call('screenshot',str(run/'survival-bricks.png'));checkpoint()
 elif a.phase=='devices':
  for c in ('gamemode creative @s','give @s minecraft:glass 6','give @s minecraft:copper_ingot 2',
            'give @s tconstruct:seared_fuel_gauge[minecraft:custom_data={"tconstruct:tank":{fluid:"minecraft:lava",amount:1000}}] 1'):cmd(c)
  # A command-supplied filled gauge is a guard fixture, not a crafted product.
  before=count('tconstruct:seared_brick');open_table()
  source=slot('tconstruct:seared_brick');click(source)
  for dst in (100,102,103,104,105):click(dst,'right')
  if g.call('menu')['carried']:click(source)
  click(slot('tconstruct:seared_fuel_gauge'));click(101)
  preview={x['slot']:x.get('stack') for x in g.call('menu')['slots']};assert not preview.get(200),preview
  g.call('screenshot',str(run/'survival-filled-gauge-rejected.png'));g.call('close');g.call('wait','10')
  filled=slots('tconstruct:seared_fuel_gauge');assert len(filled)==1 and '1000' in filled[0].get('components',''),filled
  assert count('tconstruct:seared_brick')==before,inventory();cmd('clear @s tconstruct:seared_fuel_gauge')
  brick='tconstruct:seared_brick';glass='minecraft:glass'
  craft([(brick,[100,101,102,103,105,106,107,108])],'tconstruct:seared_heater')
  craft([(brick,[100,102,106,108]),(glass,[101,103,104,105,107])],'tconstruct:seared_fuel_gauge')
  craft([(brick,[100,102,103,104,105]),('tconstruct:seared_fuel_gauge',[101])],'tconstruct:seared_melter')
  assert count('tconstruct:seared_fuel_gauge')==0
  craft([(brick,[100,102,104])],'tconstruct:seared_faucet',3)
  for _ in range(2):craft([(brick,[100,102,103,105,106,107,108])],'tconstruct:seared_basin')
  craft([(brick,[100,101,102,103,105,106,107,108]),(glass,[104])],'tconstruct:seared_fuel_tank')
  craft([(brick,[100,102,106,108]),('minecraft:copper_ingot',[103,105])],'tconstruct:seared_drain')
  merge_stacks(brick);craft([(brick,[100]*7+[101]*7+[103]*7+[104]*7)],'tconstruct:seared_bricks',many=True)
  assert count('tconstruct:seared_bricks')==7,inventory();checkpoint()
 elif a.phase=='controller':
  # Replace only our disposable fixture table/work area; every placed mod
  # block below is the item crafted in the previous phase.
  for c in ('gamemode creative @s','fill -2 71 0 2 74 4 minecraft:air','tp @s 2.5 71 0.5','gamemode survival @s'):cmd(c)
  place('tconstruct:seared_heater',0,71,2)
  place('tconstruct:seared_melter',0,72,2)
  place('tconstruct:seared_faucet',0,72,1)
  place('tconstruct:seared_basin',0,71,1)
  port=g.call('lan','open','0','commands')['port']
  guest=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=(run/'Guest').exists());guest.ready()
  for c in ('tp Guest 2.5 71 1.5','give Guest minecraft:coal 1','give Guest minecraft:copper_ingot 4'):cmd(c)
  guest.call('hotbar','9');guest.call('wait','10');guest.call('interact','0','72','2');guest.call('wait','12')
  def qslot(item):return next(s['slot'] for s in guest.call('inventory')['hotbar'] if s.get('item')==item)
  def qclick(n):guest.call('click-slot',str(n));guest.call('wait','6')
  qclick(qslot('minecraft:coal'));qclick(327);qclick(qslot('minecraft:copper_ingot'));qclick(300)
  qslots={x['slot']:x.get('stack') for x in guest.call('menu')['slots']};assert (qslots.get(300) or {}).get('item')=='minecraft:copper_ingot',qslots
  guest.call('close');guest.stop();guest.cleanup();guest=None
  g.call('wait','420',timeout=45);cmd('tp @s 2.5 71 1.5');empty_hand();g.call('interact','0','72','2');g.call('wait','12')
  state={x['slot']:x.get('stack') for x in g.call('menu')['slots']};print('MELTER AFTER GUEST',state,flush=True);assert not state.get(300),state
  g.call('screenshot',str(run/'survival-melter-lan.png'));g.call('close')
  promote('tconstruct:seared_bricks');g.call('hotbar',str(hot('tconstruct:seared_bricks')+1));g.call('wait','6');g.call('interact','0','71','1');g.call('wait','10')
  empty_hand();g.call('interact','0','72','1');g.call('wait','180',timeout=30);g.call('screenshot',str(run/'survival-controller-casting.png'));g.call('interact','0','71','1');g.call('wait','12')
  assert count('tconstruct:smeltery_controller')==1,inventory();assert count('tconstruct:seared_bricks')==6,inventory();checkpoint()
 else:raise RuntimeError('phase not yet implemented')
finally:
 if guest:guest.cleanup()
 g.cleanup()

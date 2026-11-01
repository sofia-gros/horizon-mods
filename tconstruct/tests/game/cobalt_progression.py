"""Naturally mine Cobalt, then cast and assemble using explicitly supplied fixtures."""
import argparse, re, subprocess, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'): p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--phase',choices=['mine','cast','lan','reload','blast','blast-resume'],required=True);p.add_argument('--in-frame',action='store_true')
a=p.parse_args();a.natural_world=True;a.world_seed=42
run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());world=run/'Sky'
assert (world/'Player/saves/Probe').is_dir(), 'requires existing disposable seed-42 world'
g=Game(a,world,'Player',in_frame=a.in_frame,resume=True);guest=None

def cmd(s): g.call('command',s);g.call('wait','5')
def click(s,*flags):g.call('click-slot',str(s),*flags);g.call('wait','5')
def inv():return g.call('inventory')
def count(item):return inv()['counts'].get(item,0)
def hot(item):return next(x['slot'] for x in inv()['hotbar'] if x.get('item')==item)
def empty():g.call('hotbar',str(next(x['slot'] for x in inv()['hotbar'] if not x.get('item'))+1));g.call('wait','5')
def select(item):g.call('hotbar',str(hot(item)+1));g.call('wait','5')
def node(label):
 for _ in range(120):
  n=next((n for n in g.call('ui')['widgets'] if n.get('label')==label and n.get('enabled')),None)
  if n:return n
  time.sleep(.2)
 raise AssertionError('missing '+label)
def where(dim,x,y,z):
 cmd(f'execute in minecraft:{dim} run tp @s {x+0.5} {y} {z+0.5}');g.call('wait-ready','90',timeout=95);g.call('wait','10')
def sprint(ticks):
 start=g.call('state')['ticks'];g.call('command',f'tick sprint {ticks}')
 for _ in range(360):
  time.sleep(.5)
  if g.call('state',timeout=50)['ticks']>=start+ticks:break
 else:raise AssertionError('tick sprint did not finish')
 g.call('wait','10')
def smelt(item):
 cmd('tp @s 2048.5 80 2046.5');empty();g.call('interact','2048','80','2048');g.call('wait','10');click(hot(item));click(302);sprint(2200)
 slots={x['slot']:x.get('stack') for x in g.call('menu')['slots']};assert not slots.get(302),(item,slots);g.call('close')
def take_casting(result,cast=None):
 cmd('tp @s 2050.5 80 2051.5')
 if cast:select(cast);g.call('interact','2048','79','2051');g.call('wait','8')
 empty();g.call('interact','2048','80','2051');g.call('wait','240',timeout=35);g.call('screenshot',str(run/'cobalt-part-cast.png'));before=count(result);g.call('interact','2048','79','2051');g.call('wait','10');assert count(result)==before+1,(result,inv())
try:
 g.ready()
 if a.phase=='mine':
  for c in ('gamemode creative @s','clear @s','give @s minecraft:iron_pickaxe 1','effect give @s minecraft:fire_resistance 3600 0 true'):cmd(c)
  where('overworld',6000,0,6000);outside=g.call('find-block','tconstruct:cobalt_ore','32',timeout=90);print('OVERWORLD_EXCLUSION',outside,flush=True);assert not outside['nearest'],outside
  found=None
  for x,z in [(6000,6000),(6208,6000),(6000,6208),(6400,6400)]:
   where('the_nether',x,35,z);r=g.call('find-block','tconstruct:cobalt_ore','64',timeout=90);print('SEARCH',42,x,z,r,flush=True)
   if r['nearest']:found=r['nearest'][0]['pos'];break
  assert found, 'no natural ore in bounded seed-42 search'
  for i in range(4):
   if i:
    r=g.call('find-block','tconstruct:cobalt_ore','64',timeout=90);assert r['nearest'],r;found=r['nearest'][0]['pos']
   x,y,z=found;assert g.call('block',x,y,z).get('block')=='tconstruct:cobalt_ore';print('NATURAL_ORE',42,found,flush=True)
   for c in ('gamemode creative @s',f'fill {x} {y} {z-1} {x} {y+1} {z-1} minecraft:air',f'setblock {x} {y-1} {z-1} minecraft:netherrack',f'tp @s {x+0.5} {y} {z-0.5}','gamemode survival @s'):cmd(c)
   select('minecraft:iron_pickaxe');g.call('screenshot',str(run/'cobalt-natural-ore.png'));r=g.call('break',x,y,z,'--timeout','45',timeout=50);print('MINE',r,flush=True);assert r.get('drops',{}).get('tconstruct:raw_cobalt',0)>=1,r
   for e in r.get('on_ground',[]):
    if e.get('item')=='tconstruct:raw_cobalt':
     positions=[v for v in g.call('entities')['entities'] if v.get('id')==e.get('id')]
     if positions: q=positions[0]['pos'];cmd(f'tp @s {q[0]} {q[1]} {q[2]}')
   g.call('wait','15')
  assert count('tconstruct:raw_cobalt')>=4,inv();g.call('save');g.stop();print('PASS natural cobalt ore and drops',flush=True)
 elif a.phase=='cast':
  assert count('tconstruct:raw_cobalt')>=4,inv();where('overworld',2048,80,2046)
  # All equipment, fuel, gold, wood, pattern and secondary parts are fixtures;
  # all raw Cobalt units were mined from naturally generated ore.
  for c in ('gamemode creative @s','fill 2045 79 2045 2051 82 2053 minecraft:air','fill 2045 78 2045 2051 78 2053 minecraft:stone','setblock 2048 79 2049 tconstruct:seared_bricks','setblock 2047 80 2049 tconstruct:seared_bricks','setblock 2049 80 2049 tconstruct:seared_fuel_tank','setblock 2048 80 2050 tconstruct:seared_drain[facing=south]','setblock 2048 80 2048 tconstruct:smeltery_controller[facing=north,lit=false]','setblock 2048 80 2051 tconstruct:seared_faucet[facing=south]','setblock 2048 79 2051 tconstruct:seared_table','setblock 2045 80 2048 tconstruct:part_builder','setblock 2046 80 2048 tconstruct:tinker_station','give @s minecraft:lava_bucket 1','give @s minecraft:gold_ingot 1','give @s minecraft:oak_planks 2','give @s tconstruct:pattern 1'):cmd(c)
  # Create the sacrificial wooden pick head from the real Part Builder.
  cmd('tp @s 2045.5 80 2046.5');empty();g.call('interact','2045','80','2048');g.call('wait','10');click(hot('tconstruct:pattern'));click(300);click(hot('minecraft:oak_planks'));click(301);g.call('click',node('Pick Head')['id']);g.call('wait','10');click(322);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pick_head';click(next(x['slot'] for x in inv()['hotbar'] if not x.get('item')));g.call('close')
  cmd('tp @s 2050.5 80 2049.5');select('minecraft:lava_bucket');g.call('interact','2049','80','2049');g.call('wait','8')
  smelt('minecraft:gold_ingot');take_casting('tconstruct:pick_head_cast','tconstruct:pick_head')
  smelt('tconstruct:raw_cobalt');take_casting('tconstruct:pick_head','tconstruct:pick_head_cast')
  part=next(x for x in inv()['hotbar'] if x.get('item')=='tconstruct:pick_head');assert 'tconstruct:cobalt' in part.get('components',''),part
  # Empty table to recover the reusable gold cast.
  g.call('interact','2048','79','2051');g.call('wait','8');assert count('tconstruct:pick_head_cast')>=1
  # Shape an explicitly supplied blank sand cast in the existing builder,
  # then verify that the source single-use cast is consumed on casting.
  cmd('give @s tconstruct:blank_sand_cast 1');cmd('tp @s 2045.5 80 2046.5');empty();g.call('interact','2045','80','2048');g.call('wait','10');click(hot('tconstruct:blank_sand_cast'));click(300);g.call('click',node('Pick Head')['id']);g.call('wait','10');click(322);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:pick_head_sand_cast';click(next(x['slot'] for x in inv()['hotbar'] if not x.get('item')));g.call('close')
  take_casting('tconstruct:pick_head','tconstruct:pick_head_sand_cast');assert count('tconstruct:pick_head_sand_cast')==0
  for c in ('give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1','give @s tconstruct:tool_binding[minecraft:custom_data={Material:"tconstruct:wood"}] 1','tp @s 2046.5 80 2046.5'):cmd(c)
  empty();g.call('interact','2046','80','2048');g.call('wait','10');g.call('click',node('Pickaxe')['id']);g.call('wait','10')
  for item,slot in [('tconstruct:pick_head',300),('tconstruct:tool_handle',301),('tconstruct:tool_binding',302)]:click(hot(item));click(slot)
  click(324);out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:pickaxe',out;click(next(x['slot'] for x in inv()['hotbar'] if not x.get('item')));g.call('close');g.call('screenshot',str(run/'cobalt-crafted-tool.png'))
  part=next(x for x in inv()['hotbar'] if x.get('item')=='tconstruct:pickaxe');assert 'tconstruct:cobalt' in part.get('components',''),part
  # Leave the actual assembled tool in the Station for the guest to inspect.
  empty();g.call('interact','2046','80','2048');g.call('wait','10');click(hot('tconstruct:pickaxe'));click(300);g.call('close');g.call('save')
  cmd('tp @s 2050.5 80 2046.5');port=g.call('lan','open','0','commands')['port'];guest=Game(a,world,'Guest',server='127.0.0.1:'+str(port),resume=(world/'Guest').exists());guest.ready();cmd('execute in minecraft:overworld run tp Guest 2046.5 80 2046.5');guest.call('wait','30');guest.call('hotbar','9');guest.call('interact','2046','80','2048');guest.call('wait','15')
  guest_tool=next((w.get('stack') for w in guest.call('ui')['widgets'] if w.get('id')=='slot-300'),None);assert guest_tool and 'tconstruct:cobalt' in guest_tool.get('components',''),guest_tool
  guest.call('close');guest.stop();guest.cleanup();guest=None;cmd('tp @s 2046.5 80 2046.5')
  empty();g.call('interact','2046','80','2048');g.call('wait','10');click(300);click(next(x['slot'] for x in inv()['hotbar'] if not x.get('item')));g.call('close')
  g.call('save');g.stop();print('PASS cast cobalt part, assemble and LAN',flush=True)
 elif a.phase=='lan':
  where('overworld',2046,80,2046);empty();g.call('interact','2046','80','2048');g.call('wait','12');print('HOST_STATION',g.call('ui'),flush=True);g.call('close')
  cmd('tp @s 2050.5 80 2046.5');port=g.call('lan','open','0','commands')['port'];guest=Game(a,world,'Guest',server='127.0.0.1:'+str(port),resume=(world/'Guest').exists());guest.ready()
  cmd('execute in minecraft:overworld run tp Guest 2046.5 80 2046.5');guest.call('wait-ready','90',timeout=95);guest.call('wait','30');print('GUEST_STATE',guest.call('state'),'BLOCK',guest.call('block','2046','80','2048'),'INV',guest.call('inventory'),flush=True)
  guest.call('hotbar','9');print('GUEST_INTERACT',guest.call('interact','2046','80','2048'),flush=True);guest.call('wait','20');ui=guest.call('ui');print('GUEST_UI',ui,flush=True)
  guest_tool=next((w.get('stack') for w in ui['widgets'] if w.get('id')=='slot-300'),None);assert guest_tool and 'tconstruct:cobalt' in guest_tool.get('components',''),guest_tool
  guest.call('close');guest.stop();guest.cleanup();guest=None;cmd('tp @s 2046.5 80 2046.5')
  empty();g.call('interact','2046','80','2048');g.call('wait','10');click(300);click(next(x['slot'] for x in inv()['hotbar'] if not x.get('item')));g.call('close');g.call('save');g.stop();print('PASS LAN material tool',flush=True)
 elif a.phase=='blast':
  # Obtain a fresh naturally generated ore, then use a vanilla furnace and fuel fixture.
  where('the_nether',6000,35,6000);r=g.call('find-block','tconstruct:cobalt_ore','64',timeout=90);assert r['nearest'],r
  x,y,z=next(entry['pos'] for entry in r['nearest'] if entry['pos'][1]>=35);print('BLAST_NATURAL_ORE',42,[x,y,z],flush=True)
  for c in ('gamemode creative @s',f'fill {x} {y} {z-1} {x} {y+1} {z-1} minecraft:air',f'setblock {x} {y-1} {z-1} minecraft:netherrack',f'setblock {x} {y-1} {z} minecraft:netherrack',f'tp @s {x+0.5} {y} {z-0.5}','effect give @s minecraft:fire_resistance 3600 0 true','gamemode survival @s'):cmd(c)
  select('minecraft:iron_pickaxe');before=count('tconstruct:raw_cobalt');r=g.call('break',x,y,z,'--timeout','45',timeout=50);print('BLAST_MINE',r,flush=True);g.call('wait','15');assert count('tconstruct:raw_cobalt')>before,inv()
  where('overworld',2049,79,2046)
  for c in ('gamemode creative @s','setblock 2049 79 2048 minecraft:blast_furnace','give @s minecraft:coal 1','gamemode survival @s'):cmd(c)
  empty();g.call('interact','2049','79','2048');g.call('wait','10');click(hot('tconstruct:raw_cobalt'));click(300);click(hot('minecraft:coal'));click(301);sprint(500)
  slots={x['slot']:x.get('stack') for x in g.call('menu')['slots']};print('BLAST_OUTPUT',slots,flush=True);assert (slots.get(302) or {}).get('item')=='tconstruct:cobalt_ingot',slots
  click(302);click(next(x['slot'] for x in inv()['hotbar'] if not x.get('item')));g.call('screenshot',str(run/'cobalt-blasting.png'));g.call('close');g.call('save');g.stop();print('PASS natural cobalt blast furnace',flush=True)
 elif a.phase=='blast-resume':
  assert count('tconstruct:raw_cobalt')>=1,inv();where('overworld',2049,79,2046)
  for c in ('gamemode creative @s','setblock 2049 79 2048 minecraft:blast_furnace','gamemode survival @s'):cmd(c)
  empty();g.call('interact','2049','79','2048');g.call('wait','10');click(hot('tconstruct:raw_cobalt'));click(300);click(hot('minecraft:coal'));click(301);sprint(500)
  slots={x['slot']:x.get('stack') for x in g.call('menu')['slots']};print('BLAST_OUTPUT',slots,flush=True);assert (slots.get(302) or {}).get('item')=='tconstruct:cobalt_ingot',slots
  g.call('screenshot',str(run/'cobalt-blasting.png'));click(302);click(next(x['slot'] for x in inv()['hotbar'] if not x.get('item')));g.call('close');g.call('save');g.stop();print('PASS natural cobalt blast furnace',flush=True)
 elif a.phase=='reload':
  part=next(x for x in inv()['hotbar'] if x.get('item')=='tconstruct:pickaxe');assert 'tconstruct:cobalt' in part.get('components',''),part
  cmd('tp @s 2046.5 80 2046.5');cmd('setblock 2046 80 2047 minecraft:stone');cmd('gamemode survival @s');select('tconstruct:pickaxe');g.call('break','2046','80','2047','--timeout','45',timeout=50);g.call('save');g.stop();print('PASS cobalt tool reload and mining',flush=True)
finally:
 if guest:guest.cleanup()
 g.cleanup()

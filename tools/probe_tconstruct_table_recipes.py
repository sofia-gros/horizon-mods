from pathlib import Path
from types import SimpleNamespace
import tempfile,time
from probe_horizon_server_mods import Game, ROOT
args=SimpleNamespace(game=ROOT.parent/'horizon/target/fast/horizon-client',ctl=ROOT.parent/'horizon/target/fast/horizon-ctl',resources=ROOT.parent/'horizon/.local/horizon-client/26.3',client_mod=ROOT/"Tinkers' onstruct/target/horizon-mod/tconstruct.hmod",server_mod=ROOT/"Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod")
run=Path(tempfile.mkdtemp(prefix='recipe-test-',dir=ROOT/'.local'));g=Game(args,run,'Miner',in_frame=True)
try:
 g.ready()
 for c in ['fill -2 70 -2 2 70 3 minecraft:stone','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:crafting_table','clear @s','give @s minecraft:oak_planks 8','give @s minecraft:stick 2','gamemode survival @s']:
  print(c,g.call('command',c))
 g.call('interact','0','71','2');g.call('wait','10');print('crafting table opened')
 def click(slot,*flags): g.call('click-slot',str(slot),*flags);g.call('wait','3')
 click(0);click(100,'right');click(104,'right');click(0)
 click(1);click(101,'right');click(103,'right')
 print('checking pattern recipe')
 slots={s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
 assert slots.get(200,{}).get('item')=='tconstruct:pattern', slots
 click(200,'shift');g.call('close');print('OK crafted tconstruct:pattern')
 assert g.call('inventory')['counts'].get('tconstruct:pattern')==6
 def hotbar(item): return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
 for output, pattern_slots, plank_slots in [('tconstruct:part_builder',[100,101],[103,104]),('tconstruct:tinker_station',[100,101,102],[103,105,106,108])]:
  g.call('interact','0','71','2');g.call('wait','5')
  pattern=hotbar('tconstruct:pattern');click(pattern)
  for slot in pattern_slots: click(slot,'right')
  click(pattern)
  plank=hotbar('minecraft:oak_planks');click(plank)
  for slot in plank_slots: click(slot,'right')
  if g.call('menu')['carried']: click(plank)
  slots={s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
  assert slots.get(200,{}).get('item')==output,(output,slots)
  click(200,'shift');g.call('close')
  assert g.call('inventory')['counts'].get(output)==1, output
  print('OK crafted',output)

finally: g.stop();print('run',run)

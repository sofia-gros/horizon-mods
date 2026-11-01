import argparse,sys,time
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
def hot(item):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
def choose(label):
 for _ in range(12):
  nodes=g.call('ui')['widgets']; match=next((n for n in nodes if n.get('label')==label and n.get('enabled')),None)
  if match:g.call('click',match['id']);g.call('wait','8');return
  pos=next((n['bounds'] for n in nodes if n.get('label') in ('Tool Handle','Pick Head','Hammer Head','Large Plate','Tough Handle') and n.get('bounds')),None)
  if not pos:time.sleep(.2);continue
  g.call('scroll-at',str(int(pos[0]+5)),str(int(pos[1]+5)),'-1');g.call('wait','5')
 raise AssertionError('missing Part Builder pattern: '+label)
try:
 g.ready()
 for c in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','give @s tconstruct:pattern 12','give @s minecraft:oak_planks 64'):cmd(c)
 g.call('interact','0','71','2');g.call('wait','10');click(hot('tconstruct:pattern'));click(300);click(hot('minecraft:oak_planks'));click(301)
 for label,item in [('Hammer Head','tconstruct:hammer_head'),('Tough Handle','tconstruct:tough_handle'),('Large Plate','tconstruct:large_plate'),('Large Plate','tconstruct:large_plate')]:
  choose(label);g.call('wait','10');click(322);out=g.call('menu')['carried'] or {};assert out.get('item')==item,(label,out)
  slots=g.call('inventory')['hotbar'];existing=next((s['slot'] for s in slots if s.get('item')==item),None);empty=next((s['slot'] for s in slots if not s.get('item')),None);click(existing if existing is not None else empty)
 g.call('close');inv=g.call('inventory')['counts'];assert inv.get('tconstruct:large_plate')==2,inv
 g.call('interact','1','71','2');g.call('wait','15')
 node=next(n for n in g.call('ui')['widgets'] if n.get('label')=='Sledge Hammer' and n.get('enabled'));g.call('click',node['id']);g.call('wait','10')
 click(hot('tconstruct:hammer_head'));click(300);click(hot('tconstruct:tough_handle'));click(301);click(hot('tconstruct:large_plate'));click(302,'right');click(303);g.call('wait','10');click(324)
 out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:sledge_hammer',out
 print('PASS Part Builder parts to Anvil Sledge Hammer',flush=True);g.call('close');g.call('save');g.stop()
finally:g.cleanup()

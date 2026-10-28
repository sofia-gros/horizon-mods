import argparse,sys,json,tempfile,time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from probe_horizon_server_mods import Game,ROOT
from probe_horizon_item_state import states
parser=argparse.ArgumentParser(description="Exercise part crafting, assembly and mining in an isolated game.")
parser.add_argument('--in-frame',action='store_true')
parser.add_argument('--game',type=Path,default=ROOT.parent/'horizon/target/fast/horizon-client')
parser.add_argument('--ctl',type=Path,default=ROOT.parent/'horizon/target/fast/horizon-ctl')
parser.add_argument('--resources',type=Path,default=ROOT.parent/'horizon/.local/horizon-client/26.3')
parser.add_argument('--client-mod',type=Path,default=ROOT/"Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
parser.add_argument('--server-mod',type=Path,default=ROOT/"Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod")
args=parser.parse_args(); in_frame=args.in_frame
(ROOT/'.local').mkdir(parents=True, exist_ok=True)
run=Path(tempfile.mkdtemp(prefix='generic-gui-',dir=ROOT/'.local'))
g=None
def wait(label,fn):
 for _ in range(150):
  if fn(): print('OK',label,flush=True); return
  time.sleep(.2)
 raise RuntimeError(label)
def menu(): return {s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
def hotbar(item): return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
def click(n,*args): g.call('click-slot',str(n),*args); g.call('wait','5')
try:
 g=Game(args,run,'Miner',in_frame=in_frame);g.ready()
 for command in ['fill -2 70 -2 2 70 3 minecraft:stone','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','gamemode creative @s','clear @s','give @s tconstruct:pattern 1','give @s minecraft:flint 1','give @s minecraft:oak_planks 2']:
  print('command',command,g.call('command',command),flush=True)
 wait('pattern item',lambda:any(s.get('item')=='tconstruct:pattern' for s in g.call('inventory')['hotbar']))
 g.call('command','gamemode survival @s');g.call('interact','0','71','2');g.call('wait-for','screen','station','--timeout','15')
 print('opened',g.call('menu'),flush=True);g.call('screenshot',str(run/'builder.png'),timeout=30)
 click(hotbar('tconstruct:pattern'));click(600);click(hotbar('minecraft:flint'));click(601)
 g.call('click-gui','59','23');g.call('wait','10')
 wait('insufficient material rejected',lambda:menu().get(600,{}).get('item')=='tconstruct:pattern' and menu().get(601,{}).get('count')==1 and 650 not in menu())
 g.call('close'); wait('interrupted inputs remain in station',lambda:g.call('inventory')['counts'].get('tconstruct:pattern',0)==0 and g.call('inventory')['counts'].get('minecraft:flint',0)==0)
 g.call('command','give @s minecraft:flint 1');g.call('interact','0','71','2');g.call('wait-for','screen','station','--timeout','15')
 wait('inputs survive reopening',lambda:menu().get(600,{}).get('item')=='tconstruct:pattern' and menu().get(601,{}).get('count')==1)
 click(hotbar('minecraft:flint'));click(601)
 wait('selection survives reopening',lambda:menu().get(650,{}).get('item')=='tconstruct:pick_head')
 g.call('close');g.call('save');g.stop();g=Game(args,run,'Miner',in_frame=in_frame,resume=True);g.ready()
 g.call('interact','0','71','2');g.call('wait-for','screen','station','--timeout','15')
 wait('inputs survive world reload',lambda:menu().get(600,{}).get('item')=='tconstruct:pattern' and menu().get(601,{}).get('count')==2)
 print('inserted',menu(),flush=True)
 print('ui',g.call('ui'),flush=True)
 print('click',g.call('click-gui','59','23'),flush=True);g.call('wait','10')
 print('selected',menu(),flush=True)
 wait('preview',lambda:menu().get(650,{}).get('item')=='tconstruct:pick_head')
 g.call('click-gui','59','23'); g.call('wait','10')
 wait('repeated selection retains preview',lambda:menu().get(650,{}).get('item')=='tconstruct:pick_head')
 g.call('screenshot',str(run/'part-builder-selected.png'),timeout=30)
 click(650,'shift')
 wait('pattern retained', lambda: menu().get(600,{}).get('item') == 'tconstruct:pattern' and not menu().get(601))
 click(hotbar('minecraft:oak_planks')); click(601)
 for button, item in [(77, 'tconstruct:tool_handle'), (95, 'tconstruct:tool_binding')]:
  g.call('click-gui',str(button),'23'); g.call('wait','10')
  wait(item+' preview',lambda:menu().get(650,{}).get('item')==item)
  click(650,'shift')
  wait(item+' in inventory',lambda:any(s.get('item')==item for s in g.call('inventory')['hotbar']))
 g.call('close'); wait('pattern remains in builder',lambda:g.call('inventory')['counts'].get('tconstruct:pattern',0)==0)
 g.call('command','setblock 0 71 2 tconstruct:tinker_station'); g.call('interact','0','71','2'); g.call('wait-for','screen','station','--timeout','15')
 for i,item in enumerate(['tconstruct:pick_head','tconstruct:tool_handle','tconstruct:tool_binding']):
  click(hotbar(item));click(600+i)
 wait('assembled pickaxe preview',lambda:menu().get(650,{}).get('item')=='minecraft:wooden_pickaxe')
 click(650,'shift');wait('parts consumed',lambda:all(menu().get(600+i) is None for i in range(3)))
 g.call('close'); wait('pickaxe received',lambda:any(s.get('item')=='minecraft:wooden_pickaxe' for s in g.call('inventory')['hotbar']))
 print('assembled pickaxe received',flush=True)
 g.call('save');g.stop();g=Game(args,run,'Miner',in_frame=in_frame,resume=True);g.ready()
 wait('pickaxe survives reload',lambda:any(s.get('item')=='minecraft:wooden_pickaxe' for s in g.call('inventory')['hotbar']))
 slot=hotbar('minecraft:wooden_pickaxe'); g.call('select',str(slot+1))
 wait('pickaxe selected',lambda:g.call('inventory')['held'].get('item')=='minecraft:wooden_pickaxe')
 g.call('wait','10')
 g.call('command','setblock 0 71 2 minecraft:stone')
 wait('mining target synchronized',lambda:g.call('block','0','71','2')['block']=='minecraft:stone')
 print('break result',g.call('break','0','71','2','--timeout','30',timeout=35),flush=True)
 print('after break held',g.call('inventory')['held'],flush=True)
 wait('crafted pickaxe consumes durability when used',lambda: [s['damage'] for s in states(g.call('inventory'))] == [1])
finally:
 if g: g.stop()
 print('run',run,flush=True)

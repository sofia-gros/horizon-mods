"""A broken fixed pickaxe remains and mines slowly."""
import argparse,tempfile,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true');p.add_argument('--output-parent',type=Path,default=Path('target'))
a=p.parse_args();a.output_parent.mkdir(parents=True,exist_ok=True);run=Path(tempfile.mkdtemp(prefix='broken-test-',dir=a.output_parent.resolve()));print('run',run,flush=True)
g=Game(a,run,'Player',in_frame=a.in_frame)
try:
 g.ready()
 for command in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:dirt','item replace entity @s weapon.mainhand with tconstruct:pickaxe[minecraft:damage=85]','gamemode survival @s'):g.call('command',command)
 g.call('wait','10');result=g.call('break','0','71','2','--timeout','30',timeout=35);held=g.call('inventory')['held']
 assert result['block']=='minecraft:air' and result['ticks']>=35,(result,held)
 assert held.get('item')=='tconstruct:pickaxe' and held.get('damage')==85,held
 print('PASS broken mining',result['ticks'],flush=True);g.stop()
finally:g.cleanup()

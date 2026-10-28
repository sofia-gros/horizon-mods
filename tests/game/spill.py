"""Check that destroying a station returns stored inputs."""
import argparse,tempfile,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true');p.add_argument('--output-parent',type=Path,default=Path('target'))
a=p.parse_args();a.output_parent.mkdir(parents=True,exist_ok=True);run=Path(tempfile.mkdtemp(prefix='station-spill-',dir=a.output_parent.resolve()));print('run',run,flush=True)
g=Game(a,run,'Player',in_frame=a.in_frame)
def click(i):g.call('click-slot',str(i));g.call('wait','3')
try:
 g.ready()
 for command in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','give @s tconstruct:pattern 1','give @s minecraft:flint 2'):g.call('command',command)
 time.sleep(1);g.call('interact','0','71','2');click(0);click(300);click(1);click(301);g.call('close');g.call('command','gamemode survival @s')
 result=g.call('break','0','71','2','--timeout','30',timeout=35);print(result,flush=True)
 drops=result.get('drops',{});assert drops.get('tconstruct:pattern')==1 and drops.get('minecraft:flint')==2,drops
 g.stop();print('PASS block removal spills stored inputs',flush=True)
finally:g.cleanup()

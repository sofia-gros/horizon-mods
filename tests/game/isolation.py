"""The same table position in two isolated worlds must not share contents."""
import argparse,tempfile,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true');p.add_argument('--output-parent',type=Path,default=Path('target'))
a=p.parse_args();a.output_parent.mkdir(parents=True,exist_ok=True);run=Path(tempfile.mkdtemp(prefix='station-isolation-',dir=a.output_parent.resolve()));print('run',run,flush=True)
games=[]
try:
 for name in ('First','Second'):
  g=Game(a,run,name,in_frame=a.in_frame);games.append(g);g.ready()
  for c in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder'):g.call('command',c)
  time.sleep(1);g.call('interact','0','71','2')
 first,second=games;first.call('close');first.call('command','give @s tconstruct:pattern 1');first.call('interact','0','71','2');first.call('click-slot','0');first.call('click-slot','300');first.call('wait','5')
 one={s['slot']:s['stack'] for s in first.call('menu')['slots']};two={s['slot']:s['stack'] for s in second.call('menu')['slots']}
 assert one[300]['item']=='tconstruct:pattern' and 300 not in two,(one,two)
 for g in reversed(games):g.stop()
 print('PASS independent worlds keep independent table inputs',flush=True)
finally:
 for g in reversed(games):g.cleanup()

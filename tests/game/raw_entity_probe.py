import argparse, tempfile, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true')
a=p.parse_args();run=Path(tempfile.mkdtemp(prefix='raw-entity-probe-',dir=Path('target').resolve()));print('run',run,flush=True)
g=None
try:
 g=Game(a,run,'Host',in_frame=a.in_frame);g.ready()
 for cmd in ('setblock 1 70 2 minecraft:decorated_pot', 'reload', 'setblock 2 70 2 minecraft:stone'):
  print(cmd,g.call('command',cmd),flush=True)
 g.call('wait','100');time.sleep(2);g.stop();g.cleanup();g=None
 g=Game(a,run,'Host',in_frame=a.in_frame,resume=True);g.ready();g.call('command','reload');g.call('wait','30');g.stop();g.cleanup();g=None
 log=(run/'Host/game.log').read_text()
 print('before_false',log.count('TCONSTRUCT_RAW_ENTITY_BEFORE false'),'before_true',log.count('TCONSTRUCT_RAW_ENTITY_BEFORE true'),'mesh',log.count('TCONSTRUCT_RAW_ENTITY_MESH 0 0'),flush=True)
 assert 'TCONSTRUCT_RAW_ENTITY_BEFORE false' in log, 'first raw write not reached'
 assert 'TCONSTRUCT_RAW_ENTITY_BEFORE true' in log, 'saved raw entity not found after reload'
 if a.in_frame: assert 'TCONSTRUCT_RAW_ENTITY_MESH 0 0' in log, 'in-frame mesh snapshot did not observe raw tag'
finally:
 if g is not None: g.cleanup()

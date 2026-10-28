import argparse, tempfile, time, zlib, gzip
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true')
a=p.parse_args();root=Path(tempfile.mkdtemp(prefix='material-block-place-',dir=Path('target').resolve()));print('run',root,flush=True)
def region_data():
 out=[]
 for f in (root/'Host/saves/Probe/dimensions/minecraft/overworld/region').glob('*.mca'):
  raw=f.read_bytes()
  for slot in range(1024):
   entry=int.from_bytes(raw[slot*4:slot*4+4],'big');offset=(entry>>8)*4096
   if not offset: continue
   size=int.from_bytes(raw[offset:offset+4],'big');kind=raw[offset+4];compressed=raw[offset+5:offset+4+size]
   if kind==2: out.append(zlib.decompress(compressed))
   elif kind==1: out.append(gzip.decompress(compressed))
   elif kind==3: out.append(compressed)
   else: raise AssertionError(f'unsupported compression {kind}')
 return b''.join(out)
def verify():
 raw=region_data()
 assert b'tconstruct:material_block' in raw, 'original block entity ID missing'
 assert b'\x08\x00\x08Material' + len(b'tconstruct:bronze').to_bytes(2,'big') + b'tconstruct:bronze' in raw, 'bronze material missing'
 assert b'\x08\x00\x08Material' + len(b'tconstruct:unknown').to_bytes(2,'big') + b'tconstruct:unknown' in raw, 'explicit unknown material missing'
g=None
try:
 g=Game(a,root,'Host',in_frame=a.in_frame);g.ready()
 for cmd in ('tp @s -0.5 71 0.5', 'setblock 0 70 2 minecraft:stone', 'setblock 1 70 2 minecraft:stone', 'setblock -1 70 0 minecraft:stone', 'setblock 0 70 0 minecraft:stone'): g.call('command',cmd)
 for i in range(2):
  for attempt in range(100):
   if g.call('block',str(i),'70','2').get('block') == 'minecraft:stone': break
   time.sleep(.2)
  else: raise AssertionError('support not synchronized')
 for i,material in enumerate(('tconstruct:bronze','tconstruct:unknown')):
  g.call('command',f'tp @s {i-0.5} 71 0.5')
  g.call('command','clear @s')
  print('give',material,g.call('command',f'give @s tconstruct:fake_storage_block[minecraft:custom_data={{Material:"{material}"}}] 1'),flush=True)
  g.call('wait','20');time.sleep(0.5)
  try: print('place',g.call('place',str(i),'71','2','--item','tconstruct:fake_storage_block'),flush=True)
  except Exception as e: print('place error',getattr(e,'stdout',None),getattr(e,'stderr',None),flush=True);raise
  assert g.call('block',str(i),'71','2')['block']=='tconstruct:fake_storage_block'
 g.call('wait','30');g.stop();g.cleanup();g=None
 verify();print('first save ok',flush=True)
 g=Game(a,root,'Host',in_frame=a.in_frame,resume=True);g.ready()
 for i in range(2): assert g.call('block',str(i),'71','2')['block']=='tconstruct:fake_storage_block'
 g.call('wait','20');g.stop();g.cleanup();g=None
 verify();print('save/reload ok',flush=True)
finally:
 if g is not None: g.cleanup()

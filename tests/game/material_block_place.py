import argparse, tempfile, time, zlib, gzip
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true');p.add_argument('--expect-proof',action='store_true');p.add_argument('--reverse-order',action='store_true')
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
 for cmd in ('fill -2 70 -1 4 70 5 minecraft:stone', 'tp @s -0.5 71 0.5'): g.call('command',cmd)
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
 g=Game(a,root,'Host',in_frame=a.in_frame,resume=True);g.ready()
 for cmd in ('gamemode survival @s', 'clear @s', 'give @s minecraft:diamond_pickaxe 1'):
  g.call('command',cmd)
 g.call('wait','20');time.sleep(1);g.call('select','minecraft:diamond_pickaxe')
 for i,material in enumerate(('tconstruct:bronze','tconstruct:unknown')):
  g.call('command',f'tp @s {i-0.5} 71 0.5');g.call('wait','10')
  print('break',g.call('break',str(i),'71','2','--timeout','30',timeout=35),flush=True)
  g.call('command',f'tp @s {i+0.5} 71 2.5');g.call('wait','50')
  found=False
  for attempt in range(50):
   stacks=g.call('inventory').get('hotbar',[])
   found=any(stack and stack.get('item')=='tconstruct:fake_storage_block' and material in str(stack.get('components','')) and (not a.expect_proof or material == 'tconstruct:bronze' or 'proof:17' in str(stack.get('components',''))) for stack in stacks)
   if found: break
   time.sleep(.2)
  assert found, f'missing dropped material {material}: {g.call("inventory")}'
  if a.expect_proof and material == 'tconstruct:bronze':
   assert any(stack and stack.get('item')=='minecraft:diamond' and 'proof:17' in str(stack.get('components','')) for stack in stacks), stacks
  print('drop material ok',material,flush=True)
 # Reuse the same block position with an untagged stack: no prior material
 # may leak into the next block or its drop.
 for cmd in ('gamemode creative @s', 'clear @s', 'tp @s -0.5 71 0.5', 'give @s tconstruct:fake_storage_block 1'):
  g.call('command',cmd)
 g.call('wait','20');time.sleep(1)
 g.call('place','0','71','2','--item','tconstruct:fake_storage_block')
 for cmd in ('gamemode survival @s', 'clear @s', 'give @s minecraft:diamond_pickaxe 1'):
  g.call('command',cmd)
 g.call('wait','20');time.sleep(1);g.call('select','minecraft:diamond_pickaxe')
 g.call('break','0','71','2','--timeout','30',timeout=35)
 g.call('command','tp @s 0.5 71 2.5');g.call('wait','50')
 for attempt in range(50):
  stacks=g.call('inventory').get('hotbar',[])
  drops=[stack for stack in stacks if stack and stack.get('item')=='tconstruct:fake_storage_block']
  if drops: break
  time.sleep(.2)
 else: raise AssertionError('missing untagged drop')
 assert all('Material' not in str(stack.get('components','')) for stack in drops), drops
 if a.expect_proof: assert any('proof:17' in str(stack.get('components','')) for stack in drops), drops
 print('untagged position reuse ok',flush=True)
 g.stop();g.cleanup();g=None
 if a.expect_proof:
  log=(root/'Host'/'game.log').read_text()
  order=[line.split('TCONSTRUCT_FIXTURE_DROP_ORDER ',1)[1] for line in log.splitlines() if 'TCONSTRUCT_FIXTURE_DROP_ORDER ' in line]
  expected=['tconstruct:fake_storage_block','minecraft:diamond'] if a.reverse_order else ['minecraft:diamond','tconstruct:fake_storage_block']
  assert order[:2]==expected, order
  print('independent fixture drop order ok',flush=True)
finally:
 if g is not None: g.cleanup()

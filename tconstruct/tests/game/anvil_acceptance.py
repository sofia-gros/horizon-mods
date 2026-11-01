import argparse,sys,time,zlib,gzip
from pathlib import Path
from harness import Game
from x11_text import type_text
p=argparse.ArgumentParser()
for k in ('game','ctl','resources','run_dir'):p.add_argument('--'+k.replace('_','-'),type=Path,required=True)
a=p.parse_args()
if not a.run_dir.resolve().is_relative_to((Path(__file__).resolve().parents[2] / 'target').resolve()):
 raise SystemExit('run-dir must be inside tconstruct/target')
a.run_dir.mkdir(parents=True,exist_ok=True)
g=Game(a,a.run_dir,'Player',resume=(a.run_dir/'Player').exists())
def cmd(s):g.call('command',s);g.call('wait','8')
def click(s, *opts):g.call('click-slot',str(s),*opts);g.call('wait','6')
def hot(item):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
def await_ui(label):
 for _ in range(100):
  node=next((n for n in g.call('ui')['widgets'] if n.get('label')==label and n.get('enabled')),None)
  if node:return node
  time.sleep(.2)
 raise AssertionError('missing UI '+label)
try:
 g.ready()
 for command in ('fill -2 71 -1 4 72 5 minecraft:air',):
  cmd(command)
 for x,(anvil,brick) in enumerate([('tinkers_anvil','seared_bricks'),('scorched_anvil','scorched_bricks')],1):
  for s in ('gamemode creative @s','clear @s','fill -2 70 -1 4 70 5 minecraft:stone','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:crafting_table',f'setblock {x} 71 2 minecraft:air','give @s tconstruct:slimesteel_block 3',f'give @s tconstruct:{brick} 4'):cmd(s)
  g.call('interact','0','71','2');g.call('wait','10')
  m=hot('tconstruct:slimesteel_block');click(m)
  for q in (100,101,102):click(q,'right')
  if g.call('menu')['carried']:click(m)
  b=hot('tconstruct:'+brick);click(b)
  for q in (104,106,107,108):click(q,'right')
  if g.call('menu')['carried']:click(b)
  slots={s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}; result=slots.get(200,{})
  assert result.get('item')=='tconstruct:'+anvil,(anvil,slots)
  click(200); result=g.call('menu')['carried'] or {}; assert result.get('item')=='tconstruct:'+anvil,result
  empty=next(s['slot'] for s in g.call('inventory')['hotbar'] if not s.get('item'));click(empty);g.call('close')
  cmd(f'tp @s {x+0.5} 71 0.5')
  print('placing',anvil,flush=True)
  try: g.call('place',str(x),'71','2','--item','tconstruct:'+anvil)
  except Exception as e:
   print('place failure',getattr(e,'stdout',None),getattr(e,'stderr',None),flush=True);raise
  g.call('wait','20')
  assert g.call('block',str(x),'71','2')['block']=='tconstruct:'+anvil
  g.call('hotbar','2');g.call('wait','5');g.call('interact',str(x),'71','2');g.call('wait','15')
  g.call('screenshot',str(a.run_dir/(anvil+'-empty.png')))
  node=await_ui('Sledge Hammer')
  g.call('close')
  for part,count in [('hammer_head',1),('tough_handle',1),('large_plate',2)]:
   cmd(f'give @s tconstruct:{part}[minecraft:custom_data={{Material:"tconstruct:flint"}}] {count}')
  g.call('interact',str(x),'71','2');g.call('wait','12');g.call('click',await_ui('Sledge Hammer')['id']);g.call('wait','10')
  click(hot('tconstruct:hammer_head'));click(300)
  click(hot('tconstruct:tough_handle'));click(301)
  click(hot('tconstruct:large_plate'));click(302,'right');click(303)
  g.call('wait','12');g.call('screenshot',str(a.run_dir/(anvil+'-assembly.png')))
  click(324);result=g.call('menu')['carried'] or {};assert result.get('item')=='tconstruct:sledge_hammer',result
  empty=next(s['slot'] for s in g.call('inventory')['hotbar'] if not s.get('item'));click(empty)
  g.call('close')
  for command in ('clear @s', 'item replace entity @s weapon.mainhand with tconstruct:sledge_hammer[minecraft:damage=10,minecraft:max_damage=100,minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:flint","tconstruct:flint","tconstruct:flint"],tic_max_damage:100}]', 'give @s minecraft:flint 1'):
   cmd(command)
  g.call('interact',str(x),'71','2');g.call('wait','12');click(hot('tconstruct:sledge_hammer'));click(300);click(hot('minecraft:flint'));click(305)
  g.call('wait','10');g.call('click-gui','85','10');type_text(g,'repair');g.call('wait','10')
  g.call('screenshot',str(a.run_dir/(anvil+'-repair.png')));click(325)
  result=g.call('menu')['carried'] or {};assert result.get('item')=='tconstruct:sledge_hammer' and result.get('damage',10)<10,result
  assert result.get('custom_name')=='repair',result
  click(0);g.call('close');g.call('save')
 for command in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:tinker_station'):
  cmd(command)
 g.call('interact','0','71','2');g.call('wait','15')
 assert not any(n.get('label')=='Sledge Hammer' for n in g.call('ui')['widgets']), 'large layout appeared in Station'
 g.call('close');g.call('save');g.stop();g.cleanup()
 raw=[]
 for path in (a.run_dir/'Player/saves/Probe/dimensions/minecraft/overworld/region').glob('*.mca'):
  data=path.read_bytes()
  for index in range(1024):
   entry=int.from_bytes(data[index*4:index*4+4],'big');offset=(entry>>8)*4096
   if not offset: continue
   length=int.from_bytes(data[offset:offset+4],'big'); kind=data[offset+4]; buf=data[offset+5:offset+4+length]
   raw.append(zlib.decompress(buf) if kind==2 else gzip.decompress(buf) if kind==1 else buf)
 saved=b''.join(raw)
 assert saved.count(b'tconstruct:anvil')>=2 and b'tconstruct:slimesteel_block' in saved, 'anvil metadata missing from saved world'
 g=Game(a,a.run_dir,'Player',resume=True);g.ready()
 for x,anvil in enumerate(('tinkers_anvil','scorched_anvil'),1):
  assert g.call('block',str(x),'71','2')['block']=='tconstruct:'+anvil
 for command in ('gamemode survival @s','clear @s','tp @s 1.5 71 0.5','give @s minecraft:diamond_pickaxe 1'):
  cmd(command)
 g.call('select','minecraft:diamond_pickaxe');g.call('break','1','71','2','--timeout','30',timeout=35)
 cmd('tp @s 1.5 71 2.5');g.call('wait','40')
 for _ in range(100):
  found=[s for s in g.call('inventory')['hotbar'] if s.get('item')=='tconstruct:tinkers_anvil']
  if found:break
  time.sleep(.2)
 assert found and 'slimesteel' in found[0].get('components',''),found
 cmd('gamemode creative @s');cmd('tp @s 1.5 71 0.5');g.call('place','1','71','2','--item','tconstruct:tinkers_anvil')
 g.call('wait','15');assert g.call('block','1','71','2')['block']=='tconstruct:tinkers_anvil'
 g.call('save');g.stop()
finally:g.cleanup()
print('PASS anvil crafting, placement, and GUI')

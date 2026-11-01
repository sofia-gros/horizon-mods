import argparse,sys,time,zlib,gzip
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve()), 'run-dir must be inside tconstruct/target';g=Game(a,run,'Player',resume=True)
def cmd(s):g.call('command',s);g.call('wait','7')
def click(n,*flags):g.call('click-slot',str(n),*flags);g.call('wait','7')
def hot(item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for s in ('gamemode creative @s','time set day','clear @s','effect give @s minecraft:instant_health 1 4 true','fill -2 71 -1 2 72 3 minecraft:air','fill -2 70 -1 2 70 3 minecraft:stone','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:crafting_table','give @s minecraft:basalt 4','give @s tconstruct:seared_bricks 3'):cmd(s)
 g.call('interact','0','71','2');g.call('wait','12');s=hot('tconstruct:seared_bricks');click(s)
 for i in (100,101,102):click(i,'right')
 r=hot('minecraft:basalt');click(r)
 for i in (103,105,106,108):click(i,'right')
 click(200);out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:modifier_worktable',out;click(0);g.call('close');g.call('wait','8');print('CRAFTED',g.call('inventory')['hotbar'],flush=True)
 cmd('setblock 0 71 2 minecraft:air');g.call('place','0','71','2','--item','tconstruct:modifier_worktable');g.call('wait','20');g.call('screenshot',str(run/'worktable-basalt.png'));g.call('save');g.stop();g.cleanup()
 raw=[]
 for path in (run/'Player/saves/Probe/dimensions/minecraft/overworld/region').glob('*.mca'):
  data=path.read_bytes()
  for i in range(1024):
   entry=int.from_bytes(data[i*4:i*4+4],'big');offset=(entry>>8)*4096
   if not offset:continue
   length=int.from_bytes(data[offset:offset+4],'big');kind=data[offset+4];buf=data[offset+5:offset+4+length];raw.append(zlib.decompress(buf) if kind==2 else gzip.decompress(buf) if kind==1 else buf)
 assert b'minecraft:basalt' in b''.join(raw),'rock data missing in saved chunk'
 g=Game(a,run,'Player',resume=True);g.ready();assert g.call('block','0','71','2')['block']=='tconstruct:modifier_worktable'
 for s in ('gamemode survival @s','clear @s','tp @s 0.5 71 0.5','give @s minecraft:diamond_pickaxe 1'):cmd(s)
 g.call('select','minecraft:diamond_pickaxe');g.call('break','0','71','2','--timeout','30',timeout=35);cmd('tp @s 0.5 71 2.5');g.call('wait','30')
 for _ in range(60):
  found=[x for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:modifier_worktable']
  if found:break
  time.sleep(.2)
 assert found and 'minecraft:basalt' in found[0].get('components',''),found
 print('PASS basalt save/reload/drop',found,flush=True);g.call('save');g.stop()
finally:g.cleanup()

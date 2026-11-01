"""Check supplied Ichor bottle consumption and its vanilla source effects."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for cmd in ('gamemode creative @s','clear @s','effect clear @s','tp @s 0.5 71 0.5','give @s tconstruct:ichor_slime_bottle 1','gamemode survival @s'):
  g.call('command',cmd);g.call('wait','6')
 g.call('hotbar','1');g.call('mouse','right','down');g.call('wait','40');g.call('mouse','right','up');g.call('wait','10')
 inv=g.call('inventory');print('ICHOR INVENTORY',inv,flush=True)
 assert inv['counts'].get('minecraft:glass_bottle',0)==1 and not inv['counts'].get('tconstruct:ichor_slime_bottle',0),inv
 for effect in ('levitation','slowness'):
  reply=g.call('command','effect clear @s minecraft:'+effect);print('EFFECT',effect,reply,flush=True)
  assert any('removed' in message.lower() for message in reply.get('chat',[])),reply
 g.call('save');g.stop();print('PASS supplied Ichor drinking and glass return',flush=True)
finally:g.cleanup()

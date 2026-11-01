"""Vanilla armor remains governed by native wear and break logic."""
import argparse,json
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'):p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'));p.add_argument('--in-frame',action='store_true')
a=p.parse_args();g=Game(a,a.run_dir.resolve(),'Player',in_frame=a.in_frame,resume=True)
def cmd(s):r=g.call('command',s);g.call('wait','6');return r
try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    for c in ('gamemode creative @s','effect clear @s','effect give @s minecraft:instant_health 1 4 true','clear @s','item replace entity @s armor.chest with minecraft:air','item replace entity @s armor.legs with minecraft:air','item replace entity @s armor.feet with minecraft:air','item replace entity @s armor.head with minecraft:iron_helmet[minecraft:damage=164]','gamemode survival @s'):cmd(c)
    g.call('wait','30');cmd('damage @s 4 minecraft:mob_attack');armor=g.call('inventory')['armor'];print('vanilla after',json.dumps(armor),flush=True);assert not armor[0].get('item'),armor
    g.stop();print('PASS native vanilla armor breaks',flush=True)
finally:g.cleanup()

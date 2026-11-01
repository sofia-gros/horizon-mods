"""Representative material projectile speed/spread, using synthetic stat data."""
import argparse, json, math, re
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
for name in ('game','ctl','resources'): p.add_argument('--'+name,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
p.add_argument('--in-frame',action='store_true')
a=p.parse_args(); g=Game(a,a.run_dir.resolve(),'Player',in_frame=a.in_frame,resume=True)
def cmd(s, wait=5):
    result=g.call('command',s)
    if wait: g.call('wait',str(wait))
    return result

def weapon(kind, velocity, accuracy):
    return f'tconstruct:{kind}[minecraft:custom_data={{tic_draw_speed:2.0f,tic_velocity:{velocity}f,tic_accuracy:{accuracy}f,tic_max_damage:100}},minecraft:max_damage=100]'

def fire(kind, v, accuracy):
    for s in ('gamemode creative @s', 'kill @e[type=minecraft:arrow]', 'clear @s', 'tp @s 0.5 100 0.5', 'item replace entity @s weapon.mainhand with '+weapon(kind,v,accuracy), 'give @s minecraft:arrow 8', 'gamemode survival @s'): cmd(s)
    g.call('look','0','-10');g.call('wait','30')
    g.call('mouse','right','down');g.call('wait','35');g.call('mouse','right','up')
    if kind == 'crossbow':
        g.call('wait','4');g.call('mouse','right','down');g.call('wait','1');g.call('mouse','right','up')
    data=cmd('data get entity @e[type=minecraft:arrow,sort=nearest,limit=1] Motion',0)
    values=re.findall(r'(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)d',str(data))
    assert len(values)==3,(kind,v,accuracy,data)
    vector=tuple(map(float,values)); print('shot',kind,v,accuracy,json.dumps(vector),flush=True)
    return vector
try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    for kind in ('longbow','crossbow'):
        slow=fire(kind,0.5,1.0);fast=fire(kind,1.5,1.0)
        assert math.hypot(*fast)>math.hypot(*slow)*2.0,(kind,slow,fast)
        noisy=[abs(fire(kind,1.0,0.5)[0]) for _ in range(3)]
        precise=[abs(fire(kind,1.0,1.0)[0]) for _ in range(3)]
        assert max(noisy)>max(precise)+0.005,(kind,noisy,precise)
    g.stop();print('PASS projectile material speed and spread',flush=True)
finally:g.cleanup()

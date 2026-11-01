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
    for s in ('gamemode creative @s', 'kill @e[type=minecraft:arrow]', 'clear @s', 'tp @s 20.5 100 18.5', 'item replace entity @s weapon.mainhand with '+weapon(kind,v,accuracy), 'give @s minecraft:arrow 8', 'gamemode survival @s'): cmd(s)
    g.call('look','0','-10');g.call('wait','30')
    g.call('mouse','right','down');g.call('wait','35');print('draw',kind, g.call('status').get('using'),flush=True);g.call('mouse','right','up')
    if kind == 'crossbow':
        g.call('wait','4');g.call('mouse','right','down');g.call('wait','1');g.call('mouse','right','up')
    g.call('wait','2')
    data=cmd('data get entity @e[type=minecraft:arrow,sort=nearest,limit=1] Motion',0)
    values=re.findall(r'(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)d',str(data))
    assert len(values)==3,(kind,v,accuracy,data,g.call('status'),g.call('inventory'))
    vector=tuple(map(float,values)); print('shot',kind,v,accuracy,json.dumps(vector),flush=True)
    return vector
try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    for c in ('gamemode creative @s','kill @e[tag=source_melee_test]','kill @e[tag=projectile_target]','fill 17 99 17 24 99 40 minecraft:stone','fill 17 100 17 24 110 40 minecraft:air'):cmd(c)
    for kind in ('bow','crossbow'):
        for c in ('gamemode creative @s','kill @e[type=minecraft:arrow]','clear @s','tp @s 20.5 100 18.5','item replace entity @s weapon.mainhand with minecraft:'+kind,'give @s minecraft:arrow 8','gamemode survival @s'):cmd(c)
        g.call('look','0','-10');g.call('wait','20');g.call('mouse','right','down');g.call('wait','40');using=g.call('status')['using'];g.call('mouse','right','up')
        if kind=='crossbow':g.call('wait','5');g.call('mouse','right','down');g.call('wait','1');g.call('mouse','right','up')
        g.call('wait','3');sim=g.call('simulation');arrows=[p for p in sim['projectiles'] if p['kind']=='Arrow'];inventory=g.call('inventory');print('vanilla',kind,using,arrows,inventory['counts'],flush=True)
        assert len(arrows)==1 and inventory['counts'].get('minecraft:arrow',0)==7,(kind,arrows,inventory)
    g.stop();print('PASS vanilla bows and ammunition',flush=True)
finally:g.cleanup()

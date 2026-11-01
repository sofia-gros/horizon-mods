"""Hosted guest fires material arrows; authoritative host reports motion."""
import argparse,json,math,re,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'):p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
a=p.parse_args();run=a.run_dir.resolve();games=[]
def cmd(g,s):
    r=g.call('command',s);g.call('wait','6');return r
def wait(fn):
    for _ in range(100):
        if fn():return
        time.sleep(.2)
    raise TimeoutError('LAN state')
try:
    h=Game(a,run,'Player',resume=True);games.append(h);h.ready();port=h.call('lan','open','0','commands')['port']
    q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=(run/'Guest').exists());games.append(q);q.ready()
    for c in ('gamemode creative @a','kill @e[tag=source_melee_test]','fill 27 99 27 36 99 50 minecraft:stone','fill 27 100 27 36 110 50 minecraft:air','tp @s 37.5 100 27.5','tp Guest 30.5 100 28.5'):cmd(h,c)
    for kind in ('longbow','crossbow'):
        values=[]
        for velocity in (0.5,1.5):
            cmd(h,'gamemode creative Guest');cmd(h,'kill @e[type=minecraft:arrow]');cmd(q,'clear @s')
            cmd(q,f'item replace entity @s weapon.mainhand with tconstruct:{kind}[minecraft:custom_data={{tic_draw_speed:2.0f,tic_velocity:{velocity}f,tic_accuracy:1.0f,tic_max_damage:100}},minecraft:max_damage=100]');cmd(q,'give @s minecraft:arrow 8')
            wait(lambda: (q.call('inventory')['held'] or {}).get('item')=='tconstruct:'+kind and q.call('inventory')['counts'].get('minecraft:arrow',0)==8)
            cmd(h,'gamemode survival Guest');q.call('look','0','-10');q.call('wait','20');q.call('mouse','right','down');q.call('wait','35');q.call('mouse','right','up')
            if kind=='crossbow':q.call('wait','6');q.call('mouse','right','down');q.call('wait','1');q.call('mouse','right','up')
            q.call('wait','2');data=h.call('command','data get entity @e[type=minecraft:arrow,sort=nearest,limit=1] Motion')
            v=re.findall(r'(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)d',str(data));print('LAN',kind,velocity,data,flush=True);assert len(v)==3,(kind,velocity,data,q.call('inventory'))
            values.append(math.hypot(*map(float,v)));assert q.call('inventory')['counts'].get('minecraft:arrow',0)==7
        assert values[1]>values[0]*2,(kind,values)
    q.call('screenshot',str(run/'projectile-lan.png'));q.stop();q.cleanup();games.remove(q);h.call('save');h.stop();print('PASS LAN projectile velocity',flush=True)
finally:
    for g in reversed(games):g.cleanup()

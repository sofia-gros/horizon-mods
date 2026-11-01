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
def click(slot):
    g.call('click-slot',str(slot));g.call('wait','5')
def shoot_current():
    cmd('kill @e[type=minecraft:arrow]');g.call('look','0','-10');g.call('wait','25')
    g.call('mouse','right','down');g.call('wait','35');g.call('mouse','right','up');g.call('wait','4');g.call('mouse','right','down');g.call('wait','1');g.call('mouse','right','up');g.call('wait','2')
    data=cmd('data get entity @e[type=minecraft:arrow,sort=nearest,limit=1] Motion',0)
    v=re.findall(r'(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)d',str(data));assert len(v)==3,data
    return float(v[2])
try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    for c in ('gamemode creative @s','kill @e[tag=source_melee_test]','fill 17 99 17 24 99 40 minecraft:stone','fill 17 100 17 24 110 40 minecraft:air','clear @s','tp @s 20.5 100 18.5','setblock 20 100 16 tconstruct:tinker_station','item replace entity @s weapon.mainhand with tconstruct:crossbow[minecraft:custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:string"],tic_draw_speed:1.0f,tic_velocity:1.0f,tic_accuracy:1.0f,tic_max_damage:200},minecraft:max_damage=200]','give @s tconstruct:bow_limb[minecraft:custom_data={Material:"tconstruct:cobalt"}] 1','give @s minecraft:arrow 8','gamemode survival @s'):cmd(c)
    before=shoot_current();cmd('gamemode creative @s');g.call('interact','20','100','16');g.call('wait','10');click(0);click(300)
    slot=next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')=='tconstruct:bow_limb');click(slot);click(301);click(325)
    out=g.call('menu')['carried'] or {};print('swapped output',json.dumps(out),flush=True);assert out.get('item')=='tconstruct:crossbow' and 'tconstruct:cobalt' in out.get('components',''),out
    click(0);g.call('close');g.call('wait','10');cmd('gamemode survival @s');after=shoot_current();print('swap speeds',before,after,flush=True);assert after>before*1.015,(before,after)
    g.call('save');g.stop();g.cleanup();g=Game(a,a.run_dir.resolve(),'Player',in_frame=a.in_frame,resume=True);g.ready();saved=g.call('inventory')['held'];print('saved',json.dumps(saved),flush=True);assert 'tconstruct:cobalt' in saved.get('components',''),saved
    g.stop();print('PASS material swap and saved projectile stat',flush=True)
finally:g.cleanup()

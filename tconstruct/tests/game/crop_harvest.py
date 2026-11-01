"""Crop harvest and replant representative integration cases."""
import argparse, ctypes, json, re, subprocess, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'): p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--only-break',action='store_true');p.add_argument('--start-case',type=int,default=0);p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'));p.add_argument('--in-frame',action='store_true');p.add_argument('--hammer-start',type=int,default=0);p.add_argument('--limits-only',action='store_true');p.add_argument('--smoke',action='store_true')
a=p.parse_args();run=a.run_dir.resolve();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):
    r=g.call('command',s);g.call('wait','5');return r
def screen_click(xp,yp,drag_y=None):
    tree=subprocess.check_output(['xwininfo','-root','-tree'],text=True);window=int(re.search(r'(0x[0-9a-f]+) "Horizon"',tree).group(1),16)
    x=ctypes.CDLL('libX11.so.6');xt=ctypes.CDLL('libXtst.so.6');x.XOpenDisplay.restype=ctypes.c_void_p;d=x.XOpenDisplay(None)
    x.XSetInputFocus.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong];x.XWarpPointer.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint,ctypes.c_int,ctypes.c_int];x.XFlush.argtypes=[ctypes.c_void_p];x.XCloseDisplay.argtypes=[ctypes.c_void_p];xt.XTestFakeButtonEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
    x.XSetInputFocus(d,window,1,0);x.XWarpPointer(d,0,window,0,0,0,0,xp,yp);x.XFlush(d);time.sleep(.15);xt.XTestFakeButtonEvent(d,1,1,0);x.XFlush(d);time.sleep(.1)
    if drag_y is not None:x.XWarpPointer(d,0,window,0,0,0,0,xp,drag_y);x.XFlush(d);time.sleep(.15)
    xt.XTestFakeButtonEvent(d,1,0,0);x.XFlush(d);x.XCloseDisplay(d)
scroll_positions={}
def acquire(name,start,end):
    cmd('gamemode creative @s');cmd('clear @s');g.call('open-inventory');g.call('wait','10');screen_click(596,105);g.call('wait','10')
    found=None
    for y in ([scroll_positions[name]] if name in scroll_positions else []) + list(range(start,end)):
        screen_click(649,186,drag_y=y);g.call('wait','3')
        nodes=g.call('ui')['widgets'];found=next((n for n in nodes if (n.get('stack') or {}).get('item')=='tconstruct:'+name),None)
        if found:
            scroll_positions[name]=y;break
    assert found is not None,(name,[(n.get('id'),(n.get('stack') or {}).get('item')) for n in nodes])
    i=int(found['id'].split('-')[1])-900;screen_click(285+(17+(i%9)*18)*2,136+(26+(i//9)*18)*2);g.call('wait','5')
    held=g.call('inventory')['carried'] or {};print('creative item',name,json.dumps(held),flush=True);assert held.get('item')=='tconstruct:'+name and 'tic_mining_tool' in held.get('components',''),held
    screen_click(319,376);g.call('wait','5');g.call('close');g.call('hotbar','1');g.call('wait','5')


def at(x,y,z): return g.call('block',str(x),str(y),str(z)).get('state','')
def total(item):
    inv=g.call('inventory')['counts'].get(item,0)
    ground=sum((e.get('item') or {}).get('count',0) for e in g.call('entities','10','minecraft:item')['entities'] if (e.get('item') or {}).get('item')==item)
    return inv+ground

def setup(tool,crop,age,area=False):
    acquire(tool,185,260);cmd('gamemode creative @s');cmd('time set day');cmd('weather clear')
    cmd('fill 18 99 18 22 104 22 minecraft:air');cmd('fill 18 99 18 22 99 22 minecraft:farmland[moisture=7]');cmd('setblock 18 99 20 minecraft:water')
    positions=[(x,100,z) for x in (19,20,21) for z in (19,20,21)] if area else [(20,100,20)]
    for x,y,z in positions:cmd(f'setblock {x} {y} {z} minecraft:{crop}[age={age}]')
    cmd('tp @s 20.5 100 18.5');cmd('gamemode survival @s');g.call('wait','10');return positions

def harvest(tool,crop,maximum,produce,area=False):
    positions=setup(tool,crop,maximum,area);before=g.call('inventory')['held'];count=total(produce)
    g.call('interact','20','100','20','up');g.call('wait','20');after=g.call('inventory')['held']
    states=[at(*pos) for pos in positions];delta=after.get('damage',0)-before.get('damage',0);yields=total(produce)-count
    print('harvest',tool,crop,'states',states,'wear',delta,'drops',yields,flush=True)
    assert all('age=0' in state for state in states),states
    assert delta==len(positions),(before,after)
    assert yields>=len(positions),(produce,yields)
    return positions

try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    harvest('kama','wheat',7,'minecraft:wheat')
    positions=setup('kama','wheat',0);before=g.call('inventory')['held'];g.call('look-at','20.5','100.1','20.5');assert g.call('looking-at').get('pos')==[20,100,20],g.call('looking-at');g.call('use-item');g.call('wait','20');after=g.call('inventory')['held']
    assert 'age=0' in at(*positions[0]) and before.get('damage',0)==after.get('damage',0),(before,after,at(*positions[0]))
    harvest('kama','carrots',7,'minecraft:carrot')
    harvest('scythe','wheat',7,'minecraft:wheat',True);g.call('screenshot',str(run/'crop-harvest.png'))
    # Left-click uses the ordinary authoritative block break path and its
    # source effective predicate, rather than the replant interaction.
    positions=setup('scythe','wheat',7,True);before=g.call('inventory')['held'];count=total('minecraft:wheat')
    g.call('break','20','100','20','--timeout','50',timeout=55);g.call('wait','20');after=g.call('inventory')['held']
    assert all(at(*pos)=='minecraft:air' for pos in positions),[(pos,at(*pos)) for pos in positions]
    assert after.get('damage',0)-before.get('damage',0)==9,(before,after)
    assert total('minecraft:wheat')-count==9
    print('scythe crop area break and drop',flush=True)
    positions=setup('scythe','wheat',7,True);before=g.call('inventory')['held'];maximum=before.get('damage',0)+3
    cmd('item modify entity @s weapon.mainhand {type:"minecraft:set_custom_data",function:"minecraft:set_custom_data",tag:{tic_max_damage:'+str(maximum)+'}}')
    g.call('interact','20','100','20','up');g.call('wait','20');after=g.call('inventory')['held'];states=[at(*pos) for pos in positions]
    assert sum('age=0' in state for state in states)==3 and sum('age=7' in state for state in states)==6,(states,after)
    assert after.get('damage')==maximum and 'tic_broken:1b' in after.get('components',''),after
    g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
    assert sum('age=0' in at(*pos) for pos in positions)==3
    g.stop();print('PASS crop harvest and save/reload',flush=True)
finally:g.cleanup()

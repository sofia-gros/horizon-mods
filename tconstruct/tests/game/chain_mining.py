"""Source vein and tree traversal in a saved test world."""
import argparse, ctypes, json, re, subprocess, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'): p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'));p.add_argument('--in-frame',action='store_true');p.add_argument('--hammer-start',type=int,default=0);p.add_argument('--limits-only',action='store_true');p.add_argument('--smoke',action='store_true')
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
def acquire(name,start,end):
    cmd('gamemode creative @s');cmd('clear @s');g.call('open-inventory');g.call('wait','10');screen_click(596,105);g.call('wait','10')
    found=None
    for y in range(start,end):
        screen_click(649,186,drag_y=y);g.call('wait','3')
        nodes=g.call('ui')['widgets'];found=next((n for n in nodes if (n.get('stack') or {}).get('item')=='tconstruct:'+name),None)
        if found:break
    assert found is not None,(name,[(n.get('id'),(n.get('stack') or {}).get('item')) for n in nodes])
    i=int(found['id'].split('-')[1])-900;screen_click(285+(17+(i%9)*18)*2,136+(26+(i//9)*18)*2);g.call('wait','5')
    held=g.call('inventory')['carried'] or {};print('creative item',name,json.dumps(held),flush=True);assert held.get('item')=='tconstruct:'+name and 'tic_mining_tool' in held.get('components',''),held
    screen_click(319,376);g.call('wait','5');g.call('close');g.call('hotbar','1');g.call('wait','5')

def at(p): return g.call('block',*map(str,p)).get('block')
def total(item):
    inv=g.call('inventory')['counts'].get(item,0)
    ground=sum((e.get('item') or {}).get('count',0) for e in g.call('entities','10','minecraft:item')['entities'] if (e.get('item') or {}).get('item')==item)
    return inv+ground
def setup():
    cmd('gamemode creative @s');cmd('fill 16 100 17 24 105 23 minecraft:air');cmd('fill 16 99 17 24 99 23 minecraft:stone');cmd('tp @s 20.5 100 18.5');g.call('wait','10')
def run_block(center, selected, kept, drop):
    before=g.call('inventory')['held']; baseline=total(drop);cmd('gamemode survival @s');g.call('wait','10')
    result=g.call('break',*map(str,center),'--timeout','50',timeout=55);g.call('wait','20')
    left=[(p,at(p)) for p in selected if at(p)!='minecraft:air']; hold=[(p,at(p)) for p in kept]
    after=g.call('inventory')['held']; count=total(drop)-baseline
    print('result',json.dumps(result),'remaining',left,'kept',hold,'wear',after.get('damage',0)-before.get('damage',0),'drops',count,flush=True)
    assert not left,left
    assert all(b!='minecraft:air' for _,b in hold),hold
    assert after.get('damage',0)-before.get('damage',0)==len(selected),(before,after)
    assert count==len(selected),count
def limited(name, remaining):
    acquire(name,185,260);setup()
    block='minecraft:netherrack' if name=='vein_hammer' else 'minecraft:oak_log'
    positions=[[20,101,20],[20,102,20],[20,103,20],[20,104,20]]
    for pos in positions:cmd('setblock %s %s %s '%tuple(pos)+block)
    held=g.call('inventory')['held'];maximum=held.get('damage',0)+remaining
    cmd('item modify entity @s weapon.mainhand {type:"minecraft:set_custom_data",function:"minecraft:set_custom_data",tag:{tic_max_damage:'+str(maximum)+'}}')
    count=total(block);cmd('gamemode survival @s');g.call('wait','10')
    g.call('break','20','101','20','--timeout','50',timeout=55);g.call('wait','20')
    after=g.call('inventory')['held'];missing=sum(at(p)=='minecraft:air' for p in positions)
    drops=total(block)-count
    print('limited',name,remaining,'broken blocks',missing,'drops',drops,'held',after,flush=True)
    assert missing==remaining and drops==remaining and after.get('damage')==maximum and 'tic_broken:1b' in after.get('components',''),(missing,drops,after)
try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    acquire('vein_hammer',185,260);setup()
    selected=[[20,101,20],[21,101,20],[22,101,20]];kept=[[23,101,20],[20,102,21],[19,101,20]]
    for pos in selected+kept[:2]:cmd('setblock %s %s %s minecraft:netherrack'%tuple(pos))
    cmd('setblock 19 101 20 minecraft:stone')
    run_block(selected[0],selected,kept,'minecraft:netherrack');g.call('screenshot',str(run/'chain-vein.png'))
    acquire('broad_axe',185,260);setup()
    selected=[[20,101,20],[20,102,20],[21,102,20],[22,102,20]];kept=[[20,103,21]]
    for pos in selected:cmd('setblock %s %s %s minecraft:oak_log'%tuple(pos))
    cmd('setblock 20 103 21 minecraft:birch_log')
    run_block(selected[0],selected,kept,'minecraft:oak_log');g.call('screenshot',str(run/'chain-tree.png'))
    setup();selected=[[20,y,20] for y in range(101,107)];kept=[[20,107,20]]
    for pos in selected+kept:cmd('setblock %s %s %s minecraft:oak_planks'%tuple(pos))
    run_block(selected[0],selected,kept,'minecraft:oak_planks')
    for tool in ('vein_hammer','broad_axe'):
        for remaining in (1,3):limited(tool,remaining)
    g.call('save');g.stop();g.cleanup()
    g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
    assert at([20,104,20])=='minecraft:oak_log'
    g.stop();print('PASS chain representative and reload',flush=True)
finally:g.cleanup()

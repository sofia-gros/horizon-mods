"""Creative-acquired source tools: face planes, drops, wear and filtering."""
import argparse, ctypes, json, re, subprocess, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'): p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'));p.add_argument('--in-frame',action='store_true');p.add_argument('--hammer-start',type=int,default=0);p.add_argument('--limits-only',action='store_true')
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
# Each plane is in empty test sky, offset from the saved world's existing fixtures.
def total_items(item):
    inv=g.call('inventory')['counts'].get(item,0)
    ground=sum((e.get('item') or {}).get('count',0) for e in g.call('entities','10','minecraft:item')['entities'] if (e.get('item') or {}).get('item')==item)
    return inv+ground
def scenario(block,face):
    cx,cy,cz=20,101,20
    if face[0]: lo=(cx,cy-1,cz-1);hi=(cx,cy+1,cz+1);player=(cx+face[0]*2+.5,cy-1,cz+.5)
    elif face[1]:lo=(cx-1,cy,cz-1);hi=(cx+1,cy,cz+1);player=(cx+.5,cy+1 if face[1]>0 else cy-2,cz+.5)
    else:lo=(cx-1,cy-1,cz);hi=(cx+1,cy+1,cz);player=(cx+.5,cy-1,cz+face[2]*2+.5)
    cmd('gamemode creative @s');cmd('weather clear');cmd('time set day');cmd('fill 18 99 18 22 99 22 minecraft:stone');cmd(f'fill {lo[0]} {lo[1]} {lo[2]} {hi[0]} {hi[1]} {hi[2]} {block}')
    if face[1]<0:cmd('setblock 20 99 20 minecraft:air')
    if face[1]<=0:cmd(f'setblock {int(player[0])} {int(player[1])-1} {int(player[2])} minecraft:stone')
    cmd(f'tp @s {player[0]} {player[1]} {player[2]}');g.call('wait','10');before=g.call('inventory')['held'];cmd('gamemode survival @s');g.call('wait','10')
    drop='minecraft:cobblestone' if block=='minecraft:stone' else block
    count_before=total_items(drop)
    r=g.call('break',str(cx),str(cy),str(cz),'--timeout','50',timeout=55);g.call('wait','20')
    positions=[(x,y,z) for x in range(lo[0],hi[0]+1) for y in range(lo[1],hi[1]+1) for z in range(lo[2],hi[2]+1)]
    remaining=[(p,g.call('block',*map(str,p)).get('block')) for p in positions if g.call('block',*map(str,p)).get('block')!='minecraft:air']
    after=g.call('inventory')['held'] or {};print('area',block,face,json.dumps(r),'wear',before.get('damage',0),after.get('damage',0),'remaining',remaining,flush=True)
    assert not remaining,remaining;assert after.get('damage',0)-before.get('damage',0)==9,(before,after)
    total=total_items(drop)-count_before
    print('physical drop delta',drop,total,flush=True);assert total==9,r
    cmd('gamemode creative @s');cmd('tp @s 20.5 100 20.5');g.call('wait','40')
    return r
def limits():
    cmd('gamemode creative @s');cmd('fill 18 99 18 22 99 22 minecraft:stone');cmd('fill 19 100 20 21 102 20 minecraft:netherrack')
    for x,block in ((19,'minecraft:dirt'),(20,'minecraft:stone'),(21,'minecraft:bedrock')):cmd(f'setblock {x} 100 20 {block}')
    cmd('tp @s 20.5 100 18.5');g.call('wait','10');before=g.call('inventory')['held'];cmd('gamemode survival @s');g.call('key','shift','down');g.call('wait','10')
    try:r=g.call('break','20','101','20','--timeout','50',timeout=55)
    finally:g.call('key','shift','up')
    g.call('wait','20');after=g.call('inventory')['held'];print('filtered sneak break',json.dumps(r),'wear',before.get('damage',0),after.get('damage',0),flush=True)
    assert after.get('damage',0)-before.get('damage',0)==6,(before,after)
    for x,block in ((19,'minecraft:dirt'),(20,'minecraft:stone'),(21,'minecraft:bedrock')):assert g.call('block',str(x),'100','20').get('block')==block
    for y in (101,102):
        for x in (19,20,21):assert g.call('block',str(x),str(y),'20').get('block')=='minecraft:air'
    # The source-derived tool is kept; only its custom maximum is reduced to
    # create a deterministic, explicitly test-only near-break condition.
    cmd('gamemode creative @s');maximum=after.get('damage',0)+3
    cmd('item modify entity @s weapon.mainhand {type:"minecraft:set_custom_data",function:"minecraft:set_custom_data",tag:{tic_max_damage:'+str(maximum)+'}}')
    g.call('wait','20');marked=g.call('inventory')['held'];assert 'tic_max_damage:'+str(maximum) in marked.get('components',''),marked
    cmd('fill 19 100 20 21 102 20 minecraft:netherrack');cmd('tp @s 20.5 100 18.5');cmd('gamemode survival @s');g.call('wait','10')
    result=g.call('break','20','101','20','--timeout','50',timeout=55);g.call('wait','15');held=g.call('inventory')['held'] or {}
    remains=[(x,y) for y in (100,101,102) for x in (19,20,21) if g.call('block',str(x),str(y),'20').get('block')=='minecraft:netherrack']
    print('near-break',json.dumps(result),'held',json.dumps(held),'remaining',remains,flush=True)
    assert len(remains)==6,remains;assert held.get('item')=='tconstruct:sledge_hammer' and held.get('damage')==maximum and 'tic_broken:1b' in held.get('components',''),held
    g.call('screenshot',str(run/'area-limits.png'))

try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    acquire('sledge_hammer',215,237)
    if not a.limits_only:
        for face in ([0,0,-1],[0,0,1],[-1,0,0],[1,0,0],[0,1,0],[0,-1,0])[a.hammer_start:]:scenario('minecraft:stone',face)
        g.call('screenshot',str(run/'area-hammer.png'))
    else:limits()
    if not a.limits_only:
        acquire('excavator',226,252)
        for face in ([0,0,-1],[0,0,1],[-1,0,0],[1,0,0],[0,1,0],[0,-1,0]):scenario('minecraft:dirt',face)
        g.call('screenshot',str(run/'area-excavator.png'))
    g.call('save');g.stop();print('PASS area directions or limits',flush=True)
finally:g.cleanup()

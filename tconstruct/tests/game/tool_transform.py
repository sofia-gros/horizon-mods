"""Source vein and tree traversal in a saved test world."""
import argparse, ctypes, json, re, subprocess, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'): p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--start-case',type=int,default=0);p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'));p.add_argument('--in-frame',action='store_true');p.add_argument('--hammer-start',type=int,default=0);p.add_argument('--limits-only',action='store_true');p.add_argument('--smoke',action='store_true')
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


def at(x,y,z):return g.call('block',str(x),str(y),str(z)).get('block','')
def hydrate():
    # Keep newly tilled moisture=0 farmland from randomly reverting to dirt.
    for x,z in ((17,20),(18,19),(18,21)):cmd(f'setblock {x} 100 {z} minecraft:stone')
    cmd('setblock 18 100 20 minecraft:water')
def scene(name,block,result,area=False):
    acquire(name,185,260);cmd('gamemode creative @s');cmd('time set day');cmd('weather clear')
    cmd('fill 18 100 18 22 104 22 minecraft:air');cmd('fill 18 99 18 22 99 22 minecraft:stone')
    if area:cmd('fill 19 100 19 21 100 21 '+block)
    else:cmd('setblock 20 100 20 '+block)
    if result=='minecraft:farmland':hydrate()
    cmd('tp @s 20.5 101 18.5');g.call('wait','15');before=g.call('inventory')['held'];cmd('gamemode survival @s');g.call('wait','10')
    out=g.call('interact','20','100','20','up');g.call('wait','25')
    positions=[(x,100,z) for x in (19,20,21) for z in (19,20,21)] if area else [(20,100,20)]
    for _ in range(80):
        converted=[p for p in positions if at(*p).split('[')[0]==result]
        if len(converted)==len(positions):break
        time.sleep(.2)
    after=g.call('inventory')['held']
    print('transform',name,block,result,'converted',len(converted),'wear',after.get('damage',0)-before.get('damage',0),'result',out,flush=True)
    assert len(converted)==len(positions),(name,converted,[at(*p) for p in positions])
    assert after.get('damage',0)-before.get('damage',0)==len(positions),(before,after)
    if name in ('scythe','excavator','broad_axe'):g.call('screenshot',str(run/('tool-transform-'+name+'.png')))
def center_pass_area_success():
    acquire('scythe',185,260);cmd('gamemode creative @s');cmd('fill 18 100 18 22 104 22 minecraft:air');cmd('fill 18 99 18 22 99 22 minecraft:stone');cmd('setblock 20 100 20 minecraft:stone');cmd('setblock 21 100 20 minecraft:dirt');hydrate();cmd('tp @s 20.5 101 18.5');g.call('wait','15');before=g.call('inventory')['held'];cmd('gamemode survival @s');g.call('interact','20','100','20','up');g.call('wait','20');after=g.call('inventory')['held']
    assert at(20,100,20)=='minecraft:stone' and at(21,100,20).split('[')[0]=='minecraft:farmland';assert after.get('damage',0)-before.get('damage',0)==1,(before,after)
    print('unchanged center with successful nearby transform',flush=True)
def broad_tree():
    acquire('broad_axe',185,260);cmd('gamemode creative @s');cmd('fill 18 100 18 23 104 22 minecraft:air');cmd('fill 18 99 18 23 99 22 minecraft:stone');cmd('tp @s 20.5 100 18.5')
    positions=[(20,100,20),(20,101,20),(21,101,20),(22,101,20)]
    for p in positions:cmd('setblock %s %s %s minecraft:oak_log'%p)
    cmd('gamemode survival @s');g.call('wait','10');before=g.call('inventory')['held'];g.call('interact','20','100','20','north');g.call('wait','20');after=g.call('inventory')['held']
    assert all(at(*p).split('[')[0]=='minecraft:stripped_oak_log' for p in positions),[(p,at(*p)) for p in positions]
    assert after.get('damage',0)-before.get('damage',0)==len(positions),(before,after)
    print('broad axe trunk and branch strip',flush=True)
def pathing_obstacles():
    acquire('excavator',185,260);cmd('gamemode creative @s');cmd('fill 18 100 18 22 104 22 minecraft:air');cmd('fill 18 99 18 22 99 22 minecraft:stone');cmd('fill 19 100 19 21 100 21 minecraft:grass_block');cmd('setblock 21 101 20 minecraft:short_grass');cmd('setblock 19 101 20 minecraft:stone');cmd('tp @s 20.5 101 18.5');g.call('wait','15')
    before=g.call('inventory')['held'];cmd('gamemode survival @s');g.call('interact','20','100','20','up');g.call('wait','20');after=g.call('inventory')['held']
    assert at(21,100,20).split('[')[0]=='minecraft:dirt_path' and at(21,101,20)=='minecraft:air'
    assert at(19,100,20).split('[')[0]=='minecraft:grass_block' and at(19,101,20)=='minecraft:stone'
    assert after.get('damage',0)-before.get('damage',0)==8,(before,after)
    print('source pathing replaceable and solid above',flush=True)
def broken_area():
    acquire('scythe',185,260);cmd('gamemode creative @s');cmd('fill 18 100 18 22 104 22 minecraft:air');cmd('fill 18 99 18 22 99 22 minecraft:stone');cmd('fill 19 100 19 21 100 21 minecraft:dirt');hydrate();cmd('tp @s 20.5 101 18.5')
    held=g.call('inventory')['held'];maximum=held.get('damage',0)+1;cmd('item modify entity @s weapon.mainhand {type:"minecraft:set_custom_data",function:"minecraft:set_custom_data",tag:{tic_max_damage:'+str(maximum)+'}}');cmd('gamemode survival @s');g.call('wait','10');g.call('interact','20','100','20','up');g.call('wait','20')
    after=g.call('inventory')['held'];converted=[(x,z) for x in (19,20,21) for z in (19,20,21) if at(x,100,z).split('[')[0]=='minecraft:farmland']
    assert len(converted)==1 and after.get('damage')==maximum and 'tic_broken:1b' in after.get('components',''),(converted,after)
    print('broken scythe stops area',flush=True)
def sneak_pass():
    acquire('mattock',185,260);cmd('gamemode creative @s');cmd('fill 18 100 18 22 103 22 minecraft:air');cmd('fill 18 99 18 22 99 22 minecraft:stone');cmd('setblock 20 100 20 minecraft:dirt');hydrate();cmd('tp @s 20.5 101 18.5');cmd('give @s minecraft:torch 1')
    g.call('hotbar','2');g.call('key','f','tap');g.call('hotbar','1');g.call('wait','15');cmd('gamemode survival @s');g.call('wait','10')
    before=g.call('inventory')['held'];g.call('key','shift','down')
    try:g.call('interact','20','100','20','up');g.call('wait','20')
    finally:g.call('key','shift','up')
    after=g.call('inventory')['held'];assert at(20,100,20).split('[')[0]=='minecraft:dirt';assert after.get('damage',0)==before.get('damage',0)
    assert at(20,101,20).split('[')[0]=='minecraft:torch','offhand use did not receive PASS'
    cmd('gamemode creative @s');cmd('setblock 20 101 20 minecraft:air');g.call('key','f','tap');g.call('wait','10');off=g.call('status')['offhand'];assert off and off['item']=='tconstruct:mattock',off
    cmd('gamemode survival @s');g.call('key','shift','down')
    try:g.call('interact','20','100','20','up');g.call('wait','20')
    finally:g.call('key','shift','up')
    assert at(20,100,20).split('[')[0]=='minecraft:dirt';assert g.call('status')['offhand']['damage']==off['damage']
    g.call('interact','20','100','20','up');g.call('wait','20')
    assert at(20,100,20).split('[')[0]=='minecraft:farmland';assert g.call('status')['offhand']['damage']==off['damage']+1
    print('sneak PASS main, offhand, other use and normal offhand till',flush=True)
try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    for index,(name,source,target,area) in enumerate([('mattock','minecraft:dirt','minecraft:farmland',False),('kama','minecraft:dirt','minecraft:farmland',False),('scythe','minecraft:dirt','minecraft:farmland',True),('excavator','minecraft:grass_block','minecraft:dirt_path',True),('hand_axe','minecraft:oak_log','minecraft:stripped_oak_log',False),('broad_axe','minecraft:oak_log','minecraft:stripped_oak_log',False)]):
        if index>=a.start_case:scene(name,source,target,area)
    for source,target in [('minecraft:oxidized_copper','minecraft:weathered_copper'),('minecraft:waxed_copper_block','minecraft:copper_block')]:scene('hand_axe',source,target)
    center_pass_area_success();broad_tree();pathing_obstacles();broken_area();sneak_pass()
    cmd('gamemode creative @s');cmd('clear @s');cmd('give @s minecraft:wooden_hoe 1');g.call('hotbar','1');cmd('setblock 20 100 20 minecraft:dirt');hydrate();cmd('gamemode survival @s');g.call('wait','10');before=g.call('inventory')['held'];g.call('interact','20','100','20','up');g.call('wait','20');after=g.call('inventory')['held'];assert at(20,100,20).split('[')[0]=='minecraft:farmland' and after.get('damage',0)==before.get('damage',0)+1,(before,after)
    g.call('screenshot',str(run/'tool-transform.png'));g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready();assert at(20,100,20).split('[')[0]=='minecraft:farmland';g.stop();print('PASS source tools and reload',flush=True)
finally:g.cleanup()

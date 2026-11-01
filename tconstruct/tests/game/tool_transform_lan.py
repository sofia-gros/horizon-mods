"""Normal hosted LAN area break, synchronization, drops and save/reload."""
import argparse,ctypes,json,os,re,subprocess,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'):p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
a=p.parse_args();run=a.run_dir.resolve();games=[]
def screen_click(xp,yp,drag_y=None):
    tree=subprocess.check_output(['xwininfo','-root','-tree'],text=True);window=int(re.search(r'(0x[0-9a-f]+) "Horizon"',tree).group(1),16)
    x=ctypes.CDLL('libX11.so.6');xt=ctypes.CDLL('libXtst.so.6');x.XOpenDisplay.restype=ctypes.c_void_p;d=x.XOpenDisplay(None)
    x.XSetInputFocus.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong];x.XWarpPointer.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint,ctypes.c_int,ctypes.c_int];x.XFlush.argtypes=[ctypes.c_void_p];x.XCloseDisplay.argtypes=[ctypes.c_void_p];xt.XTestFakeButtonEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
    x.XSetInputFocus(d,window,1,0);x.XWarpPointer(d,0,window,0,0,0,0,xp,yp);x.XFlush(d);time.sleep(.15);xt.XTestFakeButtonEvent(d,1,1,0);x.XFlush(d);time.sleep(.1)
    if drag_y is not None:x.XWarpPointer(d,0,window,0,0,0,0,xp,drag_y);x.XFlush(d);time.sleep(.15)
    xt.XTestFakeButtonEvent(d,1,0,0);x.XFlush(d);x.XCloseDisplay(d)
scroll_positions={}
def acquire(g,name):
    g.call('open-inventory');g.call('wait','10');screen_click(596,105);g.call('wait','10');found=None
    for y in ([scroll_positions[name]] if name in scroll_positions else []) + list(range(185,260)):
        screen_click(649,186,drag_y=y);g.call('wait','3');nodes=g.call('ui')['widgets']
        found=next((n for n in nodes if (n.get('stack') or {}).get('item')=='tconstruct:'+name),None)
        if found:
            scroll_positions[name]=y;break
    assert found is not None,[(n.get('id'),(n.get('stack') or {}).get('item')) for n in nodes]
    i=int(found['id'].split('-')[1])-900;screen_click(285+(17+(i%9)*18)*2,136+(26+(i//9)*18)*2);g.call('wait','5')
    held=g.call('inventory')['carried'] or {};assert held.get('item')=='tconstruct:'+name and 'tic_mining_tool' in held.get('components',''),held
    screen_click(319,376);g.call('wait','5');g.call('close');g.call('hotbar','1');g.call('wait','5')

def cmd(g,s):r=g.call('command',s);g.call('wait','6');return r
def wait(fn):
    for _ in range(100):
        if fn():return
        time.sleep(.2)
    raise TimeoutError('LAN state')
def total_items(g,item):
    inv=g.call('inventory')['counts'].get(item,0)
    ground=sum((e.get('item') or {}).get('count',0) for e in g.call('entities','10','minecraft:item')['entities'] if (e.get('item') or {}).get('item')==item)
    return inv+ground
try:
    h=Game(a,run,'Player',resume=True);games.append(h);h.ready();port=h.call('lan','open','0','commands')['port']
    q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=(run/'Guest').exists());games.append(q);q.ready()
    for name,source,target,area in [('mattock','minecraft:dirt','minecraft:farmland',False),('kama','minecraft:dirt','minecraft:farmland',False),('scythe','minecraft:dirt','minecraft:farmland',True),('excavator','minecraft:grass_block','minecraft:dirt_path',True),('hand_axe','minecraft:oak_log','minecraft:stripped_oak_log',False),('broad_axe','minecraft:oak_log','minecraft:stripped_oak_log',False)]:
        for command in ('gamemode creative @a','clear Guest','time set day','fill 28 100 28 32 104 32 minecraft:air','fill 28 99 28 32 99 32 minecraft:stone','tp @s 37.5 100 27.5','tp Guest 30.5 101 28.5'):cmd(h,command)
        acquire(q,name)
        cmd(h,('fill 29 100 29 31 100 31 ' if area else 'setblock 30 100 30 ')+source)
        if target=='minecraft:farmland':
            for x,z in ((27,30),(28,29),(28,31)):cmd(h,f'setblock {x} 100 {z} minecraft:stone')
            cmd(h,'setblock 28 100 30 minecraft:water')
        q.call('wait','15');before=q.call('inventory')['held'];cmd(h,'gamemode survival Guest');q.call('wait','10')
        result=q.call('interact','30','100','30','up');q.call('wait','20')
        positions=[(x,100,z) for x in (29,30,31) for z in (29,30,31)] if area else [(30,100,30)]
        wait(lambda:all(h.call('block',*map(str,p)).get('block','').split('[')[0]==target for p in positions))
        after=q.call('inventory')['held'];print('LAN transform',name,len(positions),'wear',after.get('damage',0)-before.get('damage',0),'result',result,flush=True)
        assert after.get('damage',0)-before.get('damage',0)==len(positions),(before,after)
    # A sneaking main hand must pass through to an offhand torch; swapping the
    # tool to offhand must also leave dirt and tool durability unchanged.
    for command in ('gamemode creative @a','clear Guest','fill 28 100 28 32 103 32 minecraft:air','setblock 30 100 30 minecraft:dirt','tp Guest 30.5 101 28.5'):cmd(h,command)
    acquire(q,'mattock');cmd(q,'give @s minecraft:torch 1');q.call('hotbar','2');q.call('key','f','tap');q.call('hotbar','1');q.call('wait','20');cmd(h,'gamemode survival Guest')
    before=q.call('inventory')['held'];q.call('key','shift','down');q.call('wait','10')
    try:q.call('interact','30','100','30','up');q.call('wait','20')
    finally:q.call('key','shift','up')
    wait(lambda:h.call('block','30','101','30').get('block','').split('[')[0]=='minecraft:torch')
    assert h.call('block','30','100','30').get('block','').split('[')[0]=='minecraft:dirt'
    assert q.call('inventory')['held'].get('damage',0)==before.get('damage',0)
    cmd(h,'gamemode creative Guest');cmd(h,'setblock 30 101 30 minecraft:air');q.call('key','f','tap');q.call('wait','20');off=q.call('status')['offhand'];assert off and off['item']=='tconstruct:mattock',off
    cmd(h,'gamemode survival Guest');q.call('key','shift','down');q.call('wait','10')
    try:q.call('interact','30','100','30','up');q.call('wait','20')
    finally:q.call('key','shift','up')
    assert h.call('block','30','100','30').get('block','').split('[')[0]=='minecraft:dirt';assert q.call('status')['offhand']['damage']==off['damage']
    q.call('interact','30','100','30','up');q.call('wait','20')
    wait(lambda:h.call('block','30','100','30').get('block','').split('[')[0]=='minecraft:farmland')
    assert q.call('status')['offhand']['damage']==off['damage']+1
    print('LAN normal offhand till',flush=True)
    q.call('screenshot',str(run/'tool-transform-lan.png'));q.stop();q.cleanup();games.remove(q);h.call('save');h.stop();print('PASS LAN tool transforms and sneaking',flush=True)
finally:
    for g in reversed(games):g.cleanup()

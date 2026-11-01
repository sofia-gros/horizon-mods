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
def acquire(g,name):
    g.call('open-inventory');g.call('wait','10');screen_click(596,105);g.call('wait','10');found=None
    for y in range(185,260):
        screen_click(649,186,drag_y=y);g.call('wait','3');nodes=g.call('ui')['widgets']
        found=next((n for n in nodes if (n.get('stack') or {}).get('item')=='tconstruct:'+name),None)
        if found:break
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
    for name in ('vein_hammer','broad_axe'):
      for limit in (None,1,3):
        block='minecraft:netherrack' if name=='vein_hammer' else 'minecraft:oak_log'
        for command in ('gamemode creative @a','time set day','weather clear','clear Guest','fill 26 100 27 34 105 33 minecraft:air','fill 26 99 27 34 99 33 minecraft:stone','tp @s 37.5 100 27.5','tp Guest 30.5 100 28.5'):
            cmd(h,command)
        acquire(q,name)
        positions=[(30,101+i,30) for i in range(4)]
        for pos in positions:cmd(h,'setblock %s %s %s '%pos+block)
        before=q.call('inventory')['held']
        if limit:
            maximum=before.get('damage',0)+limit
            cmd(q,'item modify entity @s weapon.mainhand {type:"minecraft:set_custom_data",function:"minecraft:set_custom_data",tag:{tic_max_damage:'+str(maximum)+'}}')
            wait(lambda: 'tic_max_damage:'+str(maximum) in (q.call('inventory')['held'] or {}).get('components',''))
        count=total_items(q,block);cmd(h,'gamemode survival Guest');q.call('wait','20')
        result=q.call('break','30','101','30','--timeout','50',timeout=55);q.call('wait','20')
        expected=limit if limit is not None else (3 if name=='vein_hammer' else 4)
        wait(lambda:sum(h.call('block',*map(str,pos)).get('block')=='minecraft:air' for pos in positions)==expected)
        after=q.call('inventory')['held'];drops=total_items(q,block)-count
        print('LAN',name,limit,json.dumps(result),'wear',after.get('damage',0)-before.get('damage',0),'drops',drops,flush=True)
        assert after.get('damage',0)-before.get('damage',0)==expected and drops==expected,(before,after,drops)
        if limit:assert 'tic_broken:1b' in after.get('components',''),after
    q.call('screenshot',str(run/'chain-lan.png'));q.stop();q.cleanup();games.remove(q);h.call('save');h.stop();print('PASS LAN chain',flush=True)
finally:
    for g in reversed(games):g.cleanup()

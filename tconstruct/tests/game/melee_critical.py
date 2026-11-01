"""Sword sprinting suppresses source sweep."""
import argparse, ctypes, json, re, subprocess, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'): p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--only-break',action='store_true');p.add_argument('--start-case',type=int,default=0);p.add_argument('--end-case',type=int,default=4);p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'));p.add_argument('--in-frame',action='store_true');p.add_argument('--hammer-start',type=int,default=0);p.add_argument('--limits-only',action='store_true');p.add_argument('--smoke',action='store_true')
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

try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    for name in ('scythe','sword'):
        acquire(name,185,260)
        for c in ('gamemode creative @s','kill @e[type=minecraft:husk,tag=source_melee_test]', 'fill 17 99 17 24 99 23 minecraft:stone','fill 17 100 17 24 105 23 minecraft:air','tp @s 20.5 101.2 18.5','summon minecraft:husk 20.5 100 20.5 {NoAI:1b,Tags:["source_melee_test"]}','summon minecraft:husk 21.3 100 20.5 {NoAI:1b,Tags:["source_melee_test"]}','gamemode survival @s'):cmd(c)
        g.call('wait','40'); g.call('look-at','20.5','100.9','20.5');g.call('command','tp @s 20.5 101.2 18.5')
        for _ in range(30):
            state=g.call('state')['player']
            if not state['on_ground'] and state['velocity'][1]<-0.08:break
            g.call('wait','1')
        else:raise AssertionError(('not falling',state))
        entities=g.call('entities','12','minecraft:husk')['entities'];target=min(entities,key=lambda e:abs(e['pos'][0]-20.5))['id'];other=next(e['id'] for e in entities if e['id']!=target)
        g.call('attack',str(target));g.call('wait','5')
        events=[e for e in g.call('hurtlog')['players'][0]['attacks'] if e['id'] in (target,other)]
        print('critical',name,json.dumps(events),flush=True)
        primary=[e for e in events if e['id']==target];second=[e for e in events if e['id']==other]
        assert len(primary)==1 and primary[0].get('critical',primary[0].get('crit')) is True,events
        if name=='scythe':assert len(second)==1 and second[0].get('critical',second[0].get('crit')) is False,events
        else:assert not second,events
    g.stop();print('PASS critical',flush=True)
finally:g.cleanup()

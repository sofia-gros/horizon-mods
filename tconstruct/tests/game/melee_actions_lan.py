"""Hosted LAN material melee action representative scenarios."""
import argparse,ctypes,json,os,re,subprocess,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'):p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--sneak-only',action='store_true');p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
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
    for command in ('gamemode creative @a','clear Guest','kill @e[type=minecraft:husk,tag=source_melee_test]',
                    'fill 27 99 27 36 99 34 minecraft:stone','fill 27 100 27 36 104 34 minecraft:air',
                    'tp @s 37.5 100 27.5','tp Guest 30.5 100 28.5'):
        cmd(h,command)
    acquire(q,'scythe')
    for x in (30.5,31.3,34.5):cmd(h,f'summon minecraft:husk {x} 100 30.5 {{NoAI:1b,Tags:["source_melee_test"]}}')
    q.call('wait','40');before={e['id']:e for e in h.call('entities','15','minecraft:husk')['entities']};assert len(before)==3,before
    target=min(before.values(),key=lambda e:abs(e['pos'][0]-30.5))['id'];other=min((e for e in before.values() if e['id']!=target),key=lambda e:e['pos'][0])['id'];far=max(before.values(),key=lambda e:e['pos'][0])['id']
    held=q.call('inventory')['held'];cmd(h,'gamemode survival Guest');q.call('wait','30');q.call('attack',str(target));q.call('wait','15')
    wait(lambda:next((e['health'] for e in h.call('entities','15','minecraft:husk')['entities'] if e['id']==other),20)<20)
    after={e['id']:e for e in h.call('entities','15','minecraft:husk')['entities']};used=q.call('inventory')['held']
    print('LAN scythe',json.dumps(before),json.dumps(after),json.dumps(used),flush=True)
    assert after[target]['health']<before[target]['health'] and after[other]['health']<before[other]['health'] and after[far]['health']==before[far]['health'],(before,after)
    assert used.get('damage',0)-held.get('damage',0)==2,(held,used)
    q.call('screenshot',str(run/'melee-scythe-lan.png'));q.stop();q.cleanup();games.remove(q);h.call('save');h.stop();print('PASS LAN scythe area and wear',flush=True)
finally:
    for g in reversed(games):g.cleanup()

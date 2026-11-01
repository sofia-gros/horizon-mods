"""Source melee area action representative cases."""
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


def entities():
    return g.call('entities','12','minecraft:husk')['entities']
def scene(tool,second_x,third_x,limited=None):
    acquire(tool,185,260)
    for command in ('gamemode creative @s','kill @e[type=minecraft:husk,tag=source_melee_test]',
                    'fill 17 99 17 26 99 24 minecraft:stone','fill 17 100 17 26 104 24 minecraft:air',
                    'tp @s 20.5 100 18.5'):
        cmd(command)
    for x in (20.5,second_x,third_x):cmd(f'summon minecraft:husk {x} 100 20.5 {{NoAI:1b,Tags:["source_melee_test"]}}')
    if limited is not None:
        held=g.call('inventory')['held'];maximum=held.get('damage',0)+limited
        cmd('item modify entity @s weapon.mainhand {type:"minecraft:set_custom_data",function:"minecraft:set_custom_data",tag:{tic_max_damage:'+str(maximum)+'}}')
    g.call('wait','40');before={e['id']:e for e in entities()};assert len(before)==3,before
    target=min(before.values(),key=lambda e:abs(e['pos'][0]-20.5))['id'];held=g.call('inventory')['held']
    cmd('gamemode survival @s');g.call('wait','30');g.call('attack',str(target));g.call('wait','8')
    after={e['id']:e for e in entities()}; tool_after=g.call('inventory')['held']; log=g.call('hurtlog')['players'][0]['attacks']
    print('MELEE',tool,'before',json.dumps(before),'after',json.dumps(after),'held',json.dumps(tool_after),'attacklog',json.dumps(log[-5:]),flush=True)
    g.call('screenshot',str(run/('melee-'+tool+'.png')))
    return before,after,held,tool_after,log,target
try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    for tool in ('scythe','kama','sword','cleaver'):
        before,after,held,used,log,target=scene(tool,21.3,24.5)
        other=min((e for e in before.values() if e['id']!=target),key=lambda e:e['pos'][0])['id']
        far=max(before.values(),key=lambda e:e['pos'][0])['id']
        assert after[target]['health']<before[target]['health'],(tool,before,after)
        assert after[other]['health']<before[other]['health'],(tool,before,after)
        assert after[far]['health']==before[far]['health'],(tool,before,after)
        cost={'scythe':2,'kama':4,'sword':1,'cleaver':1}[tool]
        assert used.get('damage',0)-held.get('damage',0)==cost,(tool,held,used)
        if tool in ('scythe','kama'):
            hits=[entry for entry in log if entry['id'] in (target,other)][-2:]
            assert len(hits)==2 and hits[0]['id']!=hits[1]['id'],hits
            assert abs(hits[0]['scale']-hits[1]['scale'])<0.0001,hits
    g.stop();print('PASS observed melee scenes',flush=True)
finally:g.cleanup()

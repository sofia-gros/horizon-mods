"""Dedicated-server spawn-protection boundary using the existing fixed world."""
import argparse, ctypes, gzip, json, os, re, shutil, socket, struct, subprocess, tempfile, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'):p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
a=p.parse_args();run=a.run_dir.resolve();world=run/'Player/saves/Probe'
eula=Path('.local/server-mods-tests/dedicated-6j20xx0n/Host/eula.txt').resolve()
if not eula.is_file() or 'eula=true' not in eula.read_text():raise RuntimeError('existing accepted dedicated test EULA is unavailable')
data=gzip.open(world/'level.dat','rb').read();start=data.index(b'\x0a\x00\x05spawn');idx=data.index(b'\x0b\x00\x03pos\x00\x00\x00\x03',start)+10
sx,sy,sz=struct.unpack('>iii',data[idx:idx+12]);print('fixed world spawn',sx,sy,sz,flush=True)

def screen_click(xp,yp,drag_y=None):
    tree=subprocess.check_output(['xwininfo','-root','-tree'],text=True);window=int(re.search(r'(0x[0-9a-f]+) "Horizon"',tree).group(1),16)
    x=ctypes.CDLL('libX11.so.6');xt=ctypes.CDLL('libXtst.so.6');x.XOpenDisplay.restype=ctypes.c_void_p;d=x.XOpenDisplay(None)
    x.XSetInputFocus.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong];x.XWarpPointer.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint,ctypes.c_int,ctypes.c_int];x.XFlush.argtypes=[ctypes.c_void_p];x.XCloseDisplay.argtypes=[ctypes.c_void_p];xt.XTestFakeButtonEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
    x.XSetInputFocus(d,window,1,0);x.XWarpPointer(d,0,window,0,0,0,0,xp,yp);x.XFlush(d);time.sleep(.15);xt.XTestFakeButtonEvent(d,1,1,0);x.XFlush(d);time.sleep(.1)
    if drag_y is not None:x.XWarpPointer(d,0,window,0,0,0,0,xp,drag_y);x.XFlush(d);time.sleep(.15)
    xt.XTestFakeButtonEvent(d,1,0,0);x.XFlush(d);x.XCloseDisplay(d)
def acquire(g):
    g.call('open-inventory');g.call('wait','10');screen_click(596,105);g.call('wait','10');found=None
    for y in range(215,237):
        screen_click(649,186,drag_y=y);g.call('wait','3');nodes=g.call('ui')['widgets']
        found=next((n for n in nodes if (n.get('stack') or {}).get('item')=='tconstruct:sledge_hammer'),None)
        if found:break
    assert found is not None,[(n.get('id'),(n.get('stack') or {}).get('item')) for n in nodes]
    i=int(found['id'].split('-')[1])-900;screen_click(285+(17+(i%9)*18)*2,136+(26+(i//9)*18)*2);g.call('wait','5')
    held=g.call('inventory')['carried'] or {};assert held.get('item')=='tconstruct:sledge_hammer' and 'tic_mining_tool' in held.get('components',''),held
    screen_click(319,376);g.call('wait','5');g.call('close');g.call('hotbar','1');g.call('wait','5')

server=None;g=None
with tempfile.TemporaryDirectory(prefix='area-server-') as temp:
    d=Path(temp);shutil.copy2(eula,d/'eula.txt')
    (d/'ops.json').write_text(json.dumps([{'uuid':'00000000-0000-4000-8000-000000000001','name':'AreaTestOperator','level':4,'bypassesPlayerLimit':False}]))
    (d/'server.properties').write_text('online-mode=false\nwhite-list=false\nenforce-secure-profile=false\nspawn-protection=2\nview-distance=3\nsimulation-distance=3\nallow-flight=true\nlevel-name=Probe\npause-when-empty-seconds=-1\n')
    sock=socket.socket();sock.bind(('127.0.0.1',0));port=sock.getsockname()[1];sock.close()
    logpath=run/'area-dedicated.log';logfile=logpath.open('w+');args=[str(a.game.resolve()),'--resources',str(a.resources.resolve()),'--dedicated','--nogui','--universe',str(world.parent),'--world','Probe','--port',str(port)]
    try:
        server=subprocess.Popen(args,cwd=d,stdin=subprocess.PIPE,stdout=logfile,stderr=subprocess.STDOUT,text=True,env=dict(os.environ,HORIZON_NO_AUDIO='1',HORIZON_NO_DISCORD='1',HORIZON_TELEMETRY_URL='off'))
        until=time.monotonic()+120
        while time.monotonic()<until:
            if server.poll() is not None:raise RuntimeError('dedicated server exited: '+logpath.read_text()[-4000:])
            if 'For help, type' in logpath.read_text():break
            time.sleep(.25)
        else:raise TimeoutError('dedicated server startup')
        g=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=(run/'Guest').exists());g.ready()
        def console(s):server.stdin.write(s+'\n');server.stdin.flush();time.sleep(.5)
        console('gamemode creative Guest');g.call('wait','20');acquire(g)
        cx,cy,cz=sx+3,101,sz
        for s in (f'fill {cx-1} {cy-1} {cz} {cx+1} {cy+1} {cz} minecraft:stone',f'setblock {cx} {cy-2} {cz-2} minecraft:stone',f'tp Guest {cx+.5} {cy-1} {cz-1.5}','gamemode survival Guest'):console(s)
        g.call('wait','30');before=g.call('inventory')['held'];result=g.call('break',str(cx),str(cy),str(cz),'--timeout','50',timeout=55);g.call('wait','20')
        remain={(x,y):g.call('block',str(x),str(y),str(cz)).get('block') for x in range(cx-1,cx+2) for y in range(cy-1,cy+2)}
        after=g.call('inventory')['held'];print('boundary',json.dumps({'result':result,'remaining':str(remain),'wear':after.get('damage',0)-before.get('damage',0)}),flush=True)
        assert all(remain[(cx-1,y)]=='minecraft:stone' for y in range(cy-1,cy+2)),remain
        assert all(remain[(x,y)]=='minecraft:air' for x in (cx,cx+1) for y in range(cy-1,cy+2)),remain
        assert after.get('damage',0)-before.get('damage',0)==6,(before,after)
        assert result.get('drops',{}).get('minecraft:cobblestone',0)==6,result
        g.call('screenshot',str(run/'area-spawn-protection.png'));g.stop();print('PASS server spawn protection boundary',flush=True)
    finally:
        if g is not None:g.cleanup()
        if server is not None and server.poll() is None:
            server.stdin.write('stop\n');server.stdin.flush()
            try:server.wait(timeout=45)
            except subprocess.TimeoutExpired:server.terminate();server.wait(timeout=10)
        logfile.close()

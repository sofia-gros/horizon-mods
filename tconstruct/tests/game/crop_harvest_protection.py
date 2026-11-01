"""Dedicated-server crop harvest protection for both hands."""
import argparse, ctypes, gzip, json, os, re, shutil, socket, struct, subprocess, tempfile, threading, time
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
    for y in range(185,260):
        screen_click(649,186,drag_y=y);g.call('wait','3');nodes=g.call('ui')['widgets']
        found=next((n for n in nodes if (n.get('stack') or {}).get('item')=='tconstruct:scythe'),None)
        if found:break
    assert found is not None,[(n.get('id'),(n.get('stack') or {}).get('item')) for n in nodes]
    i=int(found['id'].split('-')[1])-900;screen_click(285+(17+(i%9)*18)*2,136+(26+(i//9)*18)*2);g.call('wait','20')
    held=g.call('inventory')['carried'] or {}
    if not held: g.call('screenshot',str(run/'chain-spawn-acquire-debug.png')); print('creative slot',found, 'screen',g.call('status'),flush=True)
    assert held.get('item')=='tconstruct:scythe' and 'tic_mining_tool' in held.get('components',''),held
    screen_click(319,376);g.call('wait','5');g.call('close');g.call('hotbar','1');g.call('wait','5')

class EarlyStopProxy:
    # Uncompressed offline protocol: inject STOP immediately after the target
    # START to exercise the server's existing delayed-destruction branch.
    def __init__(self, server_port):
        self.listener=socket.socket();self.listener.bind(('127.0.0.1',0));self.listener.listen(1)
        self.port=self.listener.getsockname()[1];self.server_port=server_port;self.expected=None;self.injected=0
        self.thread=threading.Thread(target=self.run,daemon=True);self.thread.start()
    @staticmethod
    def varint(value):
        out=bytearray()
        while value>127:out.append((value&127)|128);value>>=7
        out.append(value);return bytes(out)
    @staticmethod
    def unpack(buf):
        value=0
        for i,b in enumerate(buf[:5]):
            value|=(b&127)<<(7*i)
            if not b&128:return value,i+1
        return None
    def run(self):
        try:
            client,_=self.listener.accept();server=socket.create_connection(('127.0.0.1',self.server_port));self.client=client;self.server=server
            def downstream():
                try:
                    while data:=server.recv(65536):client.sendall(data)
                except OSError:pass
                finally:
                    try:client.shutdown(socket.SHUT_RDWR)
                    except OSError:pass
            threading.Thread(target=downstream,daemon=True).start();buf=bytearray()
            while data:=client.recv(65536):
                buf.extend(data)
                while (header:=self.unpack(buf)) is not None:
                    length,n=header
                    if len(buf)<n+length:break
                    frame=bytes(buf[:n+length]);body=bytes(buf[n:n+length]);del buf[:n+length]
                    if self.expected is not None and len(body)>=11 and body[0]==41 and body[1]==0 and body[2:10]==self.expected:
                        stop=body[:1]+b'\x03'+body[2:];frame+=self.varint(len(stop))+stop;self.expected=None;self.injected+=1
                    server.sendall(frame)
        except OSError:pass
        finally:
            for sock in (getattr(self,'client',None),getattr(self,'server',None),self.listener):
                if sock:
                    try:sock.close()
                    except OSError:pass
    def arm(self,pos):
        x,y,z=pos;packed=((x&0x3ffffff)<<38)|((z&0x3ffffff)<<12)|(y&0xfff);self.expected=packed.to_bytes(8,'big')
    def close(self):
        for sock in (getattr(self,'client',None),getattr(self,'server',None),self.listener):
            if sock:
                try:sock.shutdown(socket.SHUT_RDWR)
                except OSError:pass
                try:sock.close()
                except OSError:pass
        self.thread.join(timeout=5)

server=None;g=None;proxy=None
with tempfile.TemporaryDirectory(prefix='area-server-') as temp:
    d=Path(temp);shutil.copy2(eula,d/'eula.txt')
    (d/'ops.json').write_text(json.dumps([{'uuid':'00000000-0000-4000-8000-000000000001','name':'AreaTestOperator','level':4,'bypassesPlayerLimit':False}]))
    (d/'server.properties').write_text('online-mode=false\nwhite-list=false\nenforce-secure-profile=false\nspawn-protection=2\nnetwork-compression-threshold=-1\nview-distance=3\nsimulation-distance=3\nallow-flight=true\nlevel-name=Probe\npause-when-empty-seconds=-1\n')
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
        proxy=EarlyStopProxy(port)
        g=Game(a,run,'Guest',server='127.0.0.1:'+str(proxy.port),resume=(run/'Guest').exists());g.ready()
        def console(s):server.stdin.write(s+'\n');server.stdin.flush();time.sleep(.5)
        console('time set day');console('weather clear');console('gamemode creative Guest');console('clear Guest');g.call('wait','20');acquire(g)
        cx,cy,cz=sx+3,100,sz
        def total_items(item):
            inv=g.call('inventory')['counts'].get(item,0)
            ground=sum((e.get('item') or {}).get('count',0) for e in g.call('entities','10','minecraft:item')['entities'] if (e.get('item') or {}).get('item')==item)
            return inv+ground
        for offhand in (False,True):
            console('gamemode creative Guest')
            for command in (f'fill {cx-2} {cy-1} {cz-2} {cx+2} {cy-1} {cz+2} minecraft:farmland[moisture=7]',f'fill {cx-1} {cy} {cz-1} {cx+1} {cy} {cz+1} minecraft:wheat[age=7]',f'fill {cx-1} {cy+1} {cz-1} {cx+1} {cy+2} {cz+1} minecraft:air',f'setblock {cx} {cy} {cz-2} minecraft:stone',f'tp Guest {cx+.5} {cy+1} {cz-1.5}'):
                console(command)
            for x,z in ((cx+3,cz),(cx+2,cz-1),(cx+2,cz+1)):console(f'setblock {x} {cy} {z} minecraft:stone')
            console(f'setblock {cx+2} {cy-1} {cz} minecraft:water')
            if offhand:
                g.call('key','f','tap');g.call('wait','20')
                assert not g.call('inventory')['held'], 'main hand must be empty before offhand test'
                assert (g.call('status')['offhand'] or {}).get('item')=='tconstruct:scythe'
            before=(g.call('status')['offhand'] if offhand else g.call('inventory')['held']);console('gamemode survival Guest');g.call('wait','25')
            result=g.call('interact',str(cx),str(cy),str(cz),'up');g.call('wait','30')
            for attempt in range(80):
                states={(x,z):g.call('block',str(x),str(cy),str(z)).get('block','') for x in range(cx-1,cx+2) for z in range(cz-1,cz+2)}
                if all('age=0' in states[(x,z)] for x in (cx,cx+1) for z in range(cz-1,cz+2)):break
                time.sleep(.2)
            after=(g.call('status')['offhand'] if offhand else g.call('inventory')['held']);print('crop boundary',offhand,states,after,result,flush=True)
            assert all('age=7' in states[(cx-1,z)] for z in range(cz-1,cz+2)),states
            assert all('age=0' in states[(x,z)] for x in (cx,cx+1) for z in range(cz-1,cz+2)),states
            assert after.get('damage',0)-before.get('damage',0)==6,(before,after)
        g.call('screenshot',str(run/'crop-harvest-protection.png'));g.stop();print('PASS server crop protection boundary and offhand',flush=True)
    finally:
        if g is not None:g.cleanup()
        if proxy is not None:proxy.close()
        if server is not None and server.poll() is None:
            server.stdin.write('stop\n');server.stdin.flush()
            try:server.wait(timeout=45)
            except subprocess.TimeoutExpired:server.terminate();server.wait(timeout=10)
        logfile.close()

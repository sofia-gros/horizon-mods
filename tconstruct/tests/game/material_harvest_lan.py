import argparse,json,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'): p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
a=p.parse_args(); run=a.run_dir.resolve(); games=[]
def cmd(g,x):
    r=g.call('command',x); g.call('wait','8'); return r
def click(g,s): g.call('click-slot',str(s)); g.call('wait','8')
def hotbar(g,item): return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
def wait(f):
    for _ in range(100):
        if f(): return
        time.sleep(.2)
    raise AssertionError('LAN synchronization timeout')
try:
    h=Game(a,run,'Player',resume=True); games.append(h); h.ready(); port=h.call('lan','open','0','commands')['port']
    q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=(run/'Guest').exists()); games.append(q); q.ready()
    for x in ('gamemode creative @a','clear @a','tp @s -0.5 71 0.5','tp Guest 0.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give Guest tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:cobalt"}] 1','give Guest tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:flint"}] 1','give Guest tconstruct:tool_binding[minecraft:custom_data={Material:"tconstruct:wood"}] 1'): cmd(h,x)
    wait(lambda:q.call('block','0','71','2').get('block')=='tconstruct:tinker_station'); q.call('wait','30'); q.call('interact','0','71','2')
    wait(lambda:any(w['id']=='slot-300' for w in q.call('ui')['widgets']))
    for i,item in enumerate(('tconstruct:pick_head','tconstruct:tool_handle','tconstruct:tool_binding')): click(q,hotbar(q,item)); click(q,300+i)
    wait(lambda:any(s['slot']==324 and (s.get('stack') or {}).get('item')=='tconstruct:pickaxe' for s in q.call('menu')['slots']))
    click(q,324); assert (q.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe'; click(q,0); q.call('close'); q.call('hotbar','1')
    cmd(h,'setblock 0 71 2 minecraft:diamond_ore'); cmd(h,'gamemode survival Guest'); q.call('wait','20')
    result=q.call('break','0','71','2','--timeout','40',timeout=45); print('guest break',json.dumps(result),flush=True); assert result.get('block')=='minecraft:air',result
    cmd(h,'tp Guest 0.5 71 2.5'); q.call('wait','60'); counts=q.call('inventory')['counts']; print('guest drops',json.dumps(counts),flush=True); assert counts.get('minecraft:diamond',0)>0,counts
    q.call('screenshot',str(run/'material-harvest-lan.png'))
    for g in reversed(games): g.stop()
    print('PASS LAN harvest drop',flush=True)
finally:
    for g in reversed(games): g.cleanup()

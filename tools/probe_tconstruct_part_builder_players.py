import argparse,tempfile,time
from pathlib import Path
from probe_horizon_server_mods import Game,ROOT
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,default=ROOT.parent/'horizon/target/fast/horizon-client');p.add_argument('--ctl',type=Path,default=ROOT.parent/'horizon/target/fast/horizon-ctl');p.add_argument('--resources',type=Path,default=ROOT.parent/'horizon/.local/horizon-client/26.3');p.add_argument('--client-mod',type=Path,default=ROOT/"Tinkers' onstruct/target/horizon-mod/tconstruct.hmod");p.add_argument('--server-mod',type=Path,default=ROOT/"Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod");args=p.parse_args();run=Path(tempfile.mkdtemp(prefix='part-shared-',dir=ROOT/'.local'));games=[]
def slots(g):return {s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
def wait(label,fn):
 for _ in range(150):
  if fn():print('OK',label,flush=True);return
  time.sleep(.2)
 raise AssertionError(label)
def click(g,n,*flags):g.call('click-slot',str(n),*flags);g.call('wait','5')
try:
 host=Game(args,run,'Host');games.append(host);host.ready();lan=host.call('lan','open','0','commands');port=lan.get('port') or host.call('lan','status').get('port');guest=Game(args,run,'Guest',server=f'127.0.0.1:{port}');games.append(guest);guest.ready()
 for cmd in ['gamemode creative @a','clear @a','fill -2 70 -2 2 70 3 minecraft:stone','tp Host -0.5 71 0.5','tp Guest 1.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','give Host tconstruct:pattern 1','give Host minecraft:flint 2','gamemode survival @a']:host.call('command',cmd)
 for g in games:
  wait(g.label+' block',lambda:g.call('block','0','71','2').get('block')=='tconstruct:part_builder');g.call('interact','0','71','2');g.call('wait-for','screen','station','--timeout','15')
 for item,target in [('tconstruct:pattern',600),('minecraft:flint',601)]:
  slot=next(s['slot'] for s in host.call('inventory')['hotbar'] if s.get('item')==item);click(host,slot);click(host,target)
 wait('guest sees inputs',lambda:slots(guest).get(601,{}).get('count')==2)
 host.call('click-gui','59','23');wait('both see head',lambda:all(slots(g).get(650,{}).get('item')=='tconstruct:pick_head' for g in games))
 guest.call('click-gui','77','23');wait('both see handle',lambda:all(slots(g).get(650,{}).get('item')=='tconstruct:tool_handle' for g in games))
 click(host,650,'shift');wait('one output and one remaining material',lambda:host.call('inventory')['counts'].get('tconstruct:tool_handle')==1 and slots(guest).get(601,{}).get('count')==1)
 guest.call('close');guest.call('interact','0','71','2');guest.call('wait-for','screen','station','--timeout','15');wait('selection and result survive reopen',lambda:slots(guest).get(650,{}).get('item')=='tconstruct:tool_handle')
 print('PASS shared selection',run,flush=True)
finally:
 for g in reversed(games):g.stop()

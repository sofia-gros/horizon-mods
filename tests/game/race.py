import sys,tempfile,time,argparse,json
from pathlib import Path
from types import SimpleNamespace
from concurrent.futures import ThreadPoolExecutor
from harness import Game
p=argparse.ArgumentParser();p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True);p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true');p.add_argument('--source-choice-wire-test',action='store_true');p.add_argument('--source-gui-test',action='store_true');p.add_argument('--output-parent',type=Path,default=Path('target'))
a=p.parse_args();a.output_parent.mkdir(parents=True,exist_ok=True);run=Path(tempfile.mkdtemp(prefix='station-race-',dir=a.output_parent.resolve()))
print('run',run,flush=True)
games=[]
def click(g,s): g.call('click-slot',str(s));g.call('wait','3')
def slots(g): return {s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
def wait(label,pred):
 for _ in range(100):
  if pred(): print('ok',label,flush=True);return
  time.sleep(.2)
 raise AssertionError(label)
def pattern(g,label):
 def node():
  return next((n for n in g.call('ui')['widgets'] if n.get('label')==label and n.get('enabled')),None)
 wait(g.label+' pattern '+label,lambda:node() is not None)
 g.call('click',node()['id']);g.call('wait','3')
try:
 h=Game(a,run,'Host',in_frame=a.in_frame);games.append(h);h.ready()
 port=h.call('lan','open','0','commands')['port']
 q=Game(a,run,'Guest',server='127.0.0.1:'+str(port));games.append(q);q.ready()
 for cmd in ('gamemode creative @a','clear @a','tp @s -0.5 71 0.5','tp Guest 1.5 71 0.5','setblock 0 71 2 tconstruct:part_builder','give @s tconstruct:pattern 2','give @s minecraft:flint 3'):
  print('command',cmd,h.call('command',cmd),flush=True)
 for g in games:
  wait(g.label+' block',lambda:g.call('block','0','71','2').get('block')=='tconstruct:part_builder')
  print(g.label,'interact',g.call('interact','0','71','2'),flush=True)
 click(h,0);click(h,300);click(h,1);click(h,301)
 wait('inputs shared',lambda:all(slots(g).get(300,{}).get('item')=='tconstruct:pattern' and slots(g).get(301,{}).get('count')==3 for g in games))
 if a.source_choice_wire_test:
  def guest_patterns():
   text=(q.root/'game.log').read_text()
   lines=[line.split('TCONSTRUCT_GUEST_BUILDER_CHOICES ',1)[1] for line in text.splitlines() if 'TCONSTRUCT_GUEST_BUILDER_CHOICES ' in line]
   return lines[-1].split(',') if lines and lines[-1] else []
  wait('guest receives source candidates',lambda:'tconstruct:pick_head' in guest_patterns() and 'tconstruct:tool_handle' in guest_patterns())
  previous=guest_patterns().index('tconstruct:pick_head')
  pack=h.root/'saves/Probe/datapacks/source-choice-wire'
  (pack/'data/tconstruct/recipes/tools/parts/builder').mkdir(parents=True)
  (pack/'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"source choice test"}}')
  recipe=json.loads((Path(__file__).resolve().parents[2]/'content/data/tconstruct/recipes/tools/parts/builder/pick_head.json').read_text())
  recipe['cost']=1
  (pack/'data/tconstruct/recipes/tools/parts/builder/pick_head.json').write_text(json.dumps(recipe))
  h.call('command','reload')
  wait('guest receives changed source order',lambda:'tconstruct:pick_head' in guest_patterns() and guest_patterns().index('tconstruct:pick_head')<previous)
 if a.source_gui_test: pattern(h,'Pick Head')
 else: click(h,324)
 assert not (h.call('menu').get('carried') or {})
 if a.source_gui_test: pattern(q,'Tool Handle')
 else: click(q,325)
 q.call('wait','20');h.call('wait','20');click(h,322)
 assert (h.call('menu').get('carried') or {}).get('item') == 'tconstruct:tool_handle', 'guest selection must replace host selection'
 click(h,2)
 # Empty the pattern slot and restore it: the source keeps the selected
 # pattern identity while its transmitted index is temporarily -1.
 click(h,300);click(h,3)
 wait('pattern removed on guest',lambda:300 not in slots(q))
 if a.source_choice_wire_test: wait('guest clears source candidates',lambda:not guest_patterns())
 click(h,3);click(h,300)
 wait('pattern restored on guest',lambda:slots(q).get(300,{}).get('item')=='tconstruct:pattern')
 if a.source_choice_wire_test: wait('guest restores source candidates',lambda:'tconstruct:pick_head' in guest_patterns())
 q.call('close');q.call('interact','0','71','2')
 wait('guest reopened table',lambda:any(s["id"] == "slot-300" for s in q.call('ui')["widgets"]))
 q.call('wait','20');time.sleep(1);q.call('screenshot',str(run/'guest-reopened.png'))
 if a.source_gui_test: pattern(q,'Pick Head')
 else: click(q,324)
 h.call('wait','20');q.call('wait','20');time.sleep(2)
 h.call('wait','20');q.call('wait','20');time.sleep(2)
 h.call('screenshot',str(run/'host-selected.png'));q.call('screenshot',str(run/'guest-selected.png'))
 with ThreadPoolExecutor(max_workers=2) as pool:
  futures=[pool.submit(click,g,322) for g in games]
  for f in futures:f.result()
 for g in games: print(g.label,'result',g.call('menu'),flush=True)
 print('outputs',sum(1 for g in games if (g.call('menu').get('carried') or {}).get('item')=='tconstruct:pick_head'),flush=True)
 assert sum(1 for g in games if (g.call('menu').get('carried') or {}).get('item')=='tconstruct:pick_head')==1
 h.call("close")
 h.call("interact","0","71","2")
 wait("host reopened table",lambda:any(s["id"] == "slot-300" for s in h.call("ui")["widgets"]))
 if a.source_gui_test and a.source_choice_wire_test:
  assert 300 not in slots(h) and slots(h).get(301,{}).get('count')==1, 'one pattern and the reloaded source cost must be consumed'
 else: assert 300 not in slots(h) and 301 not in slots(h), "pattern and material must each be consumed"
 for g in reversed(games):g.stop()
finally:
 for g in reversed(games):g.cleanup()

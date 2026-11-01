"""Check Stringy repair on the authoritative LAN Station."""
import argparse,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve()
assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve())
games=[]
def cmd(g,v):g.call('command',v);g.call('wait','8')
def click(g,s):g.call('click-slot',str(s));g.call('wait','8')
def hot(g,item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def slots(g):return {x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
try:
 h=Game(a,run,'Player',in_frame=a.in_frame,resume=True);games.append(h);h.ready()
 port=h.call('lan','open','0','commands')['port'];q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=(run/'Guest').exists());games.append(q);q.ready()
 for v in ('gamemode creative @a','clear @a','tp @s -0.5 71 0.5','tp Guest 0.5 71 0.5','setblock 0 71 2 tconstruct:tinker_station',
           'give Guest tconstruct:pickaxe[minecraft:damage=50,minecraft:max_damage=85,minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:flint","tconstruct:string"],tic_max_damage:85}] 1',
           'give Guest minecraft:string 4'):cmd(h,v)
 q.call('wait','20');q.call('interact','0','71','2');q.call('wait','15')
 for item,slot in [('tconstruct:pickaxe',300),('minecraft:string',301)]:click(q,hot(q,item));click(q,slot)
 shown=slots(q);print('LAN STRINGY OUTPUT',shown,flush=True)
 assert shown.get(325,{}).get('item')=='tconstruct:pickaxe' and shown[325].get('damage')==4,shown
 click(q,325);click(q,0);q.call('close');q.call('wait','10')
 held=q.call('inventory')['hotbar'][0];assert held.get('item')=='tconstruct:pickaxe' and held.get('damage')==4,held
 h.call('save')
 for game in reversed(games):game.stop()
 print('PASS LAN Stringy source String repair',flush=True)
finally:
 for game in reversed(games):game.cleanup()

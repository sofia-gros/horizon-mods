import argparse,sys,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve()), 'run-dir must be inside tconstruct/target';games=[]
def cmd(g,s):g.call('command',s);g.call('wait','8')
def click(g,n):g.call('click-slot',str(n));g.call('wait','8')
def hot(g,item):return next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')==item)
def wait(fn):
 for _ in range(100):
  if fn():return
  time.sleep(.2)
 raise AssertionError('timed out')
def slots(g):return {s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
try:
 h=Game(a,run,'Player',resume=True);games.append(h);h.ready();port=h.call('lan','open','0','commands')['port'];q=Game(a,run,'Guest',server='127.0.0.1:'+str(port),resume=(run/'Guest').exists());games.append(q);q.ready()
 if h.call('status')['screen']=='dead':h.call('respawn');h.call('wait','20')
 if q.call('status')['screen']=='dead':q.call('respawn');q.call('wait','20')
 for s in ('gamemode creative @a','gamerule minecraft:spawn_mobs false','clear @a','fill -2 71 -1 3 72 3 minecraft:air','fill -2 70 -1 3 70 3 minecraft:stone','tp @s -0.5 71 0.5','tp Guest 0.5 71 0.5','setblock 0 71 2 minecraft:air','time set day','give @s tconstruct:modifier_worktable[minecraft:item_model="tconstruct:modifier_worktable_basalt",minecraft:custom_data={"tconstruct:anvil":{Rock:"minecraft:basalt",Texture:"minecraft:block/basalt_side",TextureTop:"minecraft:block/basalt_top",Variant:"basalt"}}] 1','give Guest tconstruct:pickaxe[minecraft:custom_data={tic_materials:["tconstruct:flint","tconstruct:wood","tconstruct:wood"],tic_max_damage:85,tic_upgrades:{"tconstruct:netherite":1,"tconstruct:reinforced":20,"tconstruct:unbreakable":1,"tconstruct:writable":1,"tconstruct:harmonious":1,"tconstruct:forecast":1}},minecraft:max_damage=85] 1','give Guest minecraft:wet_sponge 1','give Guest tconstruct:sky_slime_crystal 1'):cmd(h,s)
 h.call('place','0','71','2','--item','tconstruct:modifier_worktable');wait(lambda:q.call('block','0','71','2').get('block')=='tconstruct:modifier_worktable');q.call('look-at','block','0','71','2');q.call('wait','20');q.call('screenshot',str(run/'unbreakable-worktable-lan.png'));q.call('interact','0','71','2');wait(lambda:any(x['id']=='slot-300' for x in q.call('ui')['widgets']))
 for item,slot in [('tconstruct:pickaxe',300),('minecraft:wet_sponge',301),('tconstruct:sky_slime_crystal',302)]:click(q,hot(q,item));click(q,slot)
 q.call('click-gui','67','19');wait(lambda:slots(q).get(325,{}).get('item')=='tconstruct:pickaxe');q.call('screenshot',str(run/'unbreakable-crystal-lan.png'));click(q,325);assert (q.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe';click(q,0);q.call('close');q.call('wait','15');inv=q.call('inventory');print('GUEST',inv,flush=True)
 picked=next(x for x in inv['hotbar'] if x.get('item')=='tconstruct:pickaxe');assert '"tconstruct:unbreakable"' not in picked.get('components',''),picked;assert inv['counts'].get('tconstruct:modifier_crystal')==1 and inv['counts'].get('minecraft:sponge')==1
 cmd(h,'setblock 0 71 2 tconstruct:scorched_anvil');wait(lambda:q.call('block','0','71','2').get('block')=='tconstruct:scorched_anvil');q.call('interact','0','71','2');wait(lambda:any(x['id']=='slot-300' for x in q.call('ui')['widgets']))
 click(q,hot(q,'tconstruct:pickaxe'));click(q,300);click(q,hot(q,'tconstruct:modifier_crystal'));click(q,305);wait(lambda:slots(q).get(325,{}).get('item')=='tconstruct:pickaxe');click(q,325);assert (q.call('menu')['carried'] or {}).get('item')=='tconstruct:pickaxe';click(q,0);q.call('close');q.call('wait','15');inv=q.call('inventory');picked=next(x for x in inv['hotbar'] if x.get('item')=='tconstruct:pickaxe');assert '"tconstruct:unbreakable":1' in picked.get('components',''),picked;assert inv['counts'].get('tconstruct:modifier_crystal',0)==0,inv
 q.stop();h.call('save');h.stop();print('PASS guest Unbreakable extraction and crystal apply',flush=True)
finally:
 for game in reversed(games):game.cleanup()

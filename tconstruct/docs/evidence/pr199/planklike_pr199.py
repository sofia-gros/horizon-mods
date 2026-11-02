import sys,types,pathlib
sys.path.insert(0,'/workspace/scratch/guide-stage-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/guide-ui-pr199');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/horizon/target/fast/horizon-client'),ctl=pathlib.Path('/workspace/horizon/target/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/guide-pr199-fixture'));g=Game(a,root,'Player')
def click(n,button=None):
 args=[str(n)] + ([button] if button else []);g.call('click-slot',*args);g.call('wait','3')
def hot(item):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
try:
 g.ready()
 for c in ['fill -2 70 -2 2 70 3 minecraft:stone','tp @s 0.5 71 0.5','setblock 0 71 3 minecraft:crafting_table']:g.call('command',c)
 for item in ['tconstruct:blazewood','tconstruct:nahuatl']:
  for result,p,w in [('tconstruct:part_builder',[100,101],[103,104]),('tconstruct:tinker_station',[100,101,102],[103,105,106,108])]:
   for c in ['clear @s','give @s tconstruct:pattern '+str(len(p)+1),'give @s '+item+' '+str(len(w)+1)]:g.call('command',c)
   g.call('wait','10');g.call('interact','0','71','3');g.call('wait','5')
   for ingredient,slots in [('tconstruct:pattern',p),(item,w)]:
    src=hot(ingredient);click(src)
    for slot in slots:click(slot,'right')
    click(src)
   state={s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')};print('CASE',item,result,'MENU',state,flush=True)
   assert state.get(200,{}).get('item')==result,(item,result,state)
   g.call('screenshot',str(root/(item.split(':')[1]+'-'+result.split(':')[1]+'.png')))
   click(200,'shift');g.call('close');g.call('wait','10');inv=g.call('inventory');print('AFTER',item,result,inv,flush=True)
   counts=inv['counts'];assert counts.get(result)==1 and counts.get(item)==1 and counts.get('tconstruct:pattern')==1,counts
 print('PASS all four crafts and unused ingredients retained',flush=True);g.stop()
finally:g.cleanup()

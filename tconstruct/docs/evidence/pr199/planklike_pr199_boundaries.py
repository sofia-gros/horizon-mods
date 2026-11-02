import sys,types,pathlib
sys.path.insert(0,'/workspace/scratch/guide-stage-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/guide-ui-pr199-boundaries');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/horizon/target/fast/horizon-client'),ctl=pathlib.Path('/workspace/horizon/target/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/guide-pr199-fixture'));g=Game(a,root,'Player')
def click(n,button=None):g.call('click-slot',str(n),*([button] if button else []));g.call('wait','2')
def hot(item):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
try:
 g.ready()
 for c in ['fill -2 70 -2 2 70 3 minecraft:stone','tp @s 0.5 71 0.5','setblock 0 71 3 minecraft:crafting_table']:g.call('command',c)
 for result,p,w in [('tconstruct:part_builder',[100,101],[103,104]),('tconstruct:tinker_station',[100,101,102],[103,105,106,108])]:
  for case in ['oak','mixed','missing-pattern','nonmember']:
   inputs=['minecraft:oak_planks']*len(w)
   if case=='mixed':inputs[-1]='tconstruct:nahuatl'
   if case=='nonmember':inputs[-1]='minecraft:dirt'
   patterns=p[:-1] if case=='missing-pattern' else p
   g.call('command','clear @s');g.call('command','give @s tconstruct:pattern '+str(len(patterns)))
   for item in set(inputs):g.call('command','give @s '+item+' '+str(inputs.count(item)))
   g.call('wait','10');g.call('interact','0','71','3');g.call('wait','5')
   for ingredient,slots in [('tconstruct:pattern',patterns)]+[(x,[w[i] for i,v in enumerate(inputs) if v==x]) for x in sorted(set(inputs))]:
    src=hot(ingredient);click(src)
    for slot in slots:click(slot,'right')
    remaining=g.call('inventory').get('carried')
    if remaining:click(src)
   state={s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')};actual=state.get(200,{}).get('item');print('CASE',result,case,'OUTPUT',actual,flush=True)
   assert actual==(result if case in ['oak','mixed'] else None),(result,case,state)
   g.call('screenshot',str(root/(result.split(':')[1]+'-'+case+'.png')))
   if actual:click(200,'shift')
   g.call('close');g.call('wait','6')
 print('PASS ordinary mixed missing and nonmember boundaries',flush=True);g.call('save');g.stop()
finally:g.cleanup()

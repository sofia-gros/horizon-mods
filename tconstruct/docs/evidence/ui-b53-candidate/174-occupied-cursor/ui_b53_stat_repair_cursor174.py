import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-pr269-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-b53-stat-repair-cursor174');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-client'),ctl=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/ui-pr265-resources'));g=Game(a,root,'Player')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==i)
def slots():return {x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:flint_and_brick[minecraft:damage=75,minecraft:max_damage=100,minecraft:custom_data={tic_max_damage:100}] 1','give @s tconstruct:seared_brick 1','give @s minecraft:dirt 1']:g.call('command',c)
 g.call('wait','10');g.call('interact','0','71','2');g.call('wait','10');click(hot('tconstruct:flint_and_brick'));click(300);click(hot('tconstruct:seared_brick'));click(301);g.call('wait','5');before=slots();assert before.get(325,{}).get('item')=='tconstruct:flint_and_brick',before;click(hot('minecraft:dirt'));assert g.call('menu')['carried']['item']=='minecraft:dirt';click(325);after=slots();carried=g.call('menu')['carried'];print('BLOCKED',json.dumps({'slots':after,'carried':carried}),flush=True);g.call('screenshot',str(root/'blocked-cursor.png'));assert carried['item']=='minecraft:dirt' and all(after.get(i)==before.get(i) for i in (300,301,325)),(before,after,carried)
 click(next(x['slot'] for x in g.call('inventory')['hotbar'] if not x.get('item')));click(325);out=g.call('menu')['carried'];print('TAKEN',json.dumps(out),flush=True);g.call('screenshot',str(root/'taken.png'));assert out['item']=='tconstruct:flint_and_brick' and out['damage']==0,out;g.call('close');g.stop();print('PASS occupied cursor rejects and preserves ingredients',flush=True)
finally:g.cleanup()

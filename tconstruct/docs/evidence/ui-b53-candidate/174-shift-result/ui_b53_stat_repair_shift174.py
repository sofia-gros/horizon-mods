import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-pr269-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-b53-stat-repair-shift174');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-client'),ctl=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/ui-pr265-resources'));g=Game(a,root,'Player')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==i)
def slots():return {x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')}
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:flint_and_brick[minecraft:damage=75,minecraft:max_damage=100,minecraft:custom_data={tic_max_damage:100}] 1','give @s tconstruct:seared_brick 1','give @s minecraft:dirt 1']:g.call('command',c)
 g.call('wait','10');g.call('interact','0','71','2');g.call('wait','10');click(hot('tconstruct:flint_and_brick'));click(300);click(hot('tconstruct:seared_brick'));click(301);g.call('wait','5');before=slots();assert before.get(325,{}).get('item')=='tconstruct:flint_and_brick',before;g.call('click-slot','325','shift');g.call('wait','10');after=slots();inv=g.call('inventory');print('SHIFT',json.dumps({'slots':after,'inventory':inv}),flush=True);g.call('screenshot',str(root/'shift-result.png'));tool=next(x for x in inv['hotbar']+inv['main'] if x.get('item')=='tconstruct:flint_and_brick');assert tool.get('damage',0)==0 and 300 not in after and 301 not in after and 325 not in after,(after,tool);g.call('close');g.stop();print('PASS shift-take repair result',flush=True)
finally:g.cleanup()

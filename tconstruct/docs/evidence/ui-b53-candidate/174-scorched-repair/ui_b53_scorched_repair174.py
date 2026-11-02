import sys,types,pathlib
sys.path.insert(0,'/workspace/scratch/ui-workstation-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-b53-scorched-repair174');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-client'),ctl=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/ui-pr265-resources'));g=Game(a,root,'Player')
def click(n,button=None):g.call('click-slot',str(n),*([button] if button else []));g.call('wait','7')
def hot(i):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==i)
def slots():return {s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:flint_and_brick[minecraft:damage=75,minecraft:max_damage=100,minecraft:custom_data={tic_max_damage:100}] 1','give @s minecraft:dirt 1','give @s tconstruct:scorched_brick 1']:g.call('command',c)
 g.call('wait','15');g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:flint_and_brick'));click(300);click(hot('minecraft:dirt'));click(301);print('WRONG',slots(),flush=True);g.call('screenshot',str(root/'wrong-input.png'));assert 325 not in slots();click(301);click(next(x['slot'] for x in g.call('inventory')['hotbar'] if not x.get('item')));click(hot('tconstruct:scorched_brick'));click(301);g.call('wait','10');s=slots();print('BACON',s,flush=True);g.call('screenshot',str(root/'bacon-output.png'));assert s.get(325,{}).get('item')=='tconstruct:flint_and_brick',s;click(325);out=g.call('menu').get('carried') or {};print('OUTPUT',out,flush=True);assert out.get('item')=='tconstruct:flint_and_brick' and out.get('damage',75)<75,out;g.call('screenshot',str(root/'repaired-carried.png'));g.call('close');g.stop();print('PASS station wrong-input rejection and scorched brick repair output',flush=True)
finally:g.cleanup()

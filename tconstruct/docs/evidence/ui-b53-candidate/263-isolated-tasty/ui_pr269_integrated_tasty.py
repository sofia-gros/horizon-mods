import sys,types,pathlib
sys.path.insert(0,'/workspace/scratch/ui-workstation-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-pr269-integrated-tasty');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-pr269-isolated-client'),ctl=pathlib.Path('/workspace/scratch/ui-pr269-isolated-ctl'),resources=pathlib.Path('/workspace/scratch/ui-pr269-final-resources'));g=Game(a,root,'Player')
def click(n,button=None):g.call('click-slot',str(n),*([button] if button else []));g.call('wait','7')
def hot(i):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==i)
def slots():return {s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:pickaxe[minecraft:damage=100,minecraft:max_damage=500,minecraft:custom_data={tic_max_damage:500,tic_materials:["tconstruct:pig_iron","tconstruct:wood","tconstruct:wood"]}] 1','give @s minecraft:dirt 1','give @s tconstruct:bacon 1']:g.call('command',c)
 g.call('wait','15');g.call('interact','0','71','2');g.call('wait','12');click(hot('tconstruct:pickaxe'));click(300);click(hot('minecraft:dirt'));click(301);print('WRONG',slots(),flush=True);g.call('screenshot',str(root/'wrong-input.png'));assert 325 not in slots();click(301);click(next(x['slot'] for x in g.call('inventory')['hotbar'] if not x.get('item')));click(hot('tconstruct:bacon'));click(301);g.call('wait','10');s=slots();print('BACON',s,flush=True);g.call('screenshot',str(root/'bacon-output.png'));assert s.get(325,{}).get('item')=='tconstruct:pickaxe',s;click(325);out=g.call('menu').get('carried') or {};print('OUTPUT',out,flush=True);assert out.get('item')=='tconstruct:pickaxe' and out.get('damage',100)<100,out;g.call('screenshot',str(root/'repaired-carried.png'));g.call('close');g.stop();print('PASS station wrong-input rejection and bacon repair output',flush=True)
finally:g.cleanup()

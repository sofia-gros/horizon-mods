import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-pr269-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-b53-stat-malformed174');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-client'),ctl=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/ui-pr265-resources'));g=Game(a,root,'Player')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==i)
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station']:g.call('command',c)
 for name,damage,extra in [('malformed',75,',tic_materials:17')]:
  g.call('command','clear @s');g.call('command',f'give @s tconstruct:flint_and_brick[minecraft:damage={damage},minecraft:max_damage=100,minecraft:custom_data={{tic_max_damage:100{extra}}}] 1');g.call('command','give @s tconstruct:seared_brick 1');g.call('wait','10');g.call('hotbar',str(hot('tconstruct:flint_and_brick')+1));g.call('wait','5');held=g.call('inventory')['held'];print('INITIAL_HELD',json.dumps(held),flush=True);assert 'tic_materials:17' in held.get('components',''),held;g.call('interact','0','71','2');g.call('wait','10');click(hot('tconstruct:flint_and_brick'));click(300);click(hot('tconstruct:seared_brick'));click(301);g.call('wait','5');rows={x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')};print('REJECT',name,json.dumps(rows),flush=True);g.call('screenshot',str(root/(name+'.png')));assert 325 not in rows and rows.get(300,{}).get('item')=='tconstruct:flint_and_brick' and rows.get(301,{}).get('item')=='tconstruct:seared_brick',rows;g.call('close');g.call('wait','5')
 g.stop();print('PASS undamaged and malformed inputs rejected',flush=True)
finally:g.cleanup()

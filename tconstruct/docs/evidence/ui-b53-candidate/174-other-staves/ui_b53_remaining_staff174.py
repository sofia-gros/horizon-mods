import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-pr269-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-b53-remaining-staff174');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-client'),ctl=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/ui-pr265-resources'));g=Game(a,root,'Player')
def click(n):g.call('click-slot',str(n));g.call('wait','7')
def hot(i):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==i)
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:tinker_station']:g.call('command',c)
 for name in ('sky_staff','ender_staff','ichor_staff'):
  item='tconstruct:'+name;g.call('command','clear @s');g.call('command',f'give @s {item}[minecraft:damage=400,minecraft:max_damage=800,minecraft:custom_data={{tic_max_damage:800}}] 1');g.call('command','give @s tconstruct:greenheart_planks 1');g.call('wait','10');g.call('interact','0','71','2');g.call('wait','10');click(hot(item));click(300);click(hot('tconstruct:greenheart_planks'));click(301);rows={x['slot']:x['stack'] for x in g.call('menu')['slots'] if x.get('stack')};print('PREVIEW',name,rows,flush=True);g.call('screenshot',str(root/(name+'-preview.png')));assert rows.get(325,{}).get('item')==item,rows;click(325);out=g.call('menu').get('carried') or {};print('OUTPUT',name,json.dumps(out),flush=True);assert out.get('item')==item and out.get('damage',400)<400,out;click(0);g.call('close');g.call('wait','5')
 g.stop();print('PASS sky ender ichor staff repair',flush=True)
finally:g.cleanup()

import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-workstation-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-b53-melter-be-normal');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-client'),ctl=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/ui-b53-resources'));g=Game(a,root,'Player')
def hot(item):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
def select(item):g.call('hotbar',str(hot(item)+1));g.call('wait','8')
def empty():g.call('hotbar',str(next(s['slot'] for s in g.call('inventory')['hotbar'] if not s.get('item'))+1));g.call('wait','8')
def click(n):g.call('click-slot',str(n));g.call('wait','5')
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','setblock 2 70 1 minecraft:stone','tp @s 2.5 71 1.5','setblock 0 71 2 tconstruct:seared_heater']:g.call('command',c)
 empty();g.call('interact','0','71','2');g.call('wait','5');assert g.call('status')['screen']=='inventory';g.call('close')
 for c in ['setblock 0 72 2 tconstruct:seared_melter','setblock 0 72 1 tconstruct:seared_faucet[facing=north]','setblock 0 71 1 tconstruct:seared_table','give @s minecraft:coal 2','give @s minecraft:copper_ingot 10']:g.call('command',c)
 g.call('wait','10');g.call('interact','0','72','2');g.call('wait','10');assert any(w.get('id')=='slot-327' for w in g.call('ui')['widgets']);print('BEFORE',json.dumps(g.call('menu')),flush=True);click(hot('minecraft:coal'));print('COAL_PICK',json.dumps(g.call('menu')),flush=True);click(327);print('COAL_INSERT',json.dumps(g.call('menu')),flush=True);click(hot('minecraft:copper_ingot'));click(300);print('COPPER_INSERT',json.dumps(g.call('menu')),flush=True);print('BE_BEFORE',json.dumps(g.call('command','data get block 0 72 2')),flush=True);print('HEATER_BEFORE',json.dumps(g.call('command','data get block 0 71 2')),flush=True)
 for _ in range(1):g.call('wait','400',timeout=45)
 print('STATUS',json.dumps(g.call('status')),flush=True);print('UI_AFTER',json.dumps(g.call('ui')),flush=True);menu=g.call('menu');print('MELTER',json.dumps(menu),flush=True);g.call('screenshot',str(root/'melter-filled.png'));g.call('close');g.call('wait','5');g.call('interact','0','72','2');g.call('wait','10');print('REOPEN',json.dumps(g.call('menu')),flush=True);print('BE_AFTER',json.dumps(g.call('command','data get block 0 72 2')),flush=True);print('HEATER_AFTER',json.dumps(g.call('command','data get block 0 71 2')),flush=True);g.call('screenshot',str(root/'reopen.png'));g.call('close')
 g.stop()
finally:g.cleanup()

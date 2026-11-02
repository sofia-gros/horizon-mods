import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-pr269-isolated-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-pr269-melter-can-click');a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-pr269-isolated-client'),ctl=pathlib.Path('/workspace/scratch/ui-pr269-isolated-ctl'),resources=pathlib.Path('/workspace/scratch/ui-pr269-final-resources'));g=Game(a,root,'Player',resume=True)
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','tp @s 2.5 71 1.5','give @s tconstruct:copper_can 1']:g.call('command',c)
 g.call('wait','10');g.call('interact','0','72','2');g.call('wait','15');slot=next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')=='tconstruct:copper_can');g.call('click-slot',str(slot));g.call('wait','10');print('BEFORE',json.dumps(g.call('menu')),flush=True);g.call('screenshot',str(root/'before.png'));g.call('click-gui','100','60');g.call('wait','15');print('AFTER',json.dumps(g.call('menu')),flush=True);g.call('screenshot',str(root/'after.png'));g.call('click-slot',str(slot));g.call('close');g.call('wait','10');inv=g.call('inventory');print('CAN',json.dumps(inv),flush=True);g.call('save');g.stop()
finally:g.cleanup()

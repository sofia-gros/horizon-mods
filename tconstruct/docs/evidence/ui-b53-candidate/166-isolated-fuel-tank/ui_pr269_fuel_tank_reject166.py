import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-workstation-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-pr269-fuel-tank-reject166');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-pr269-isolated-client'),ctl=pathlib.Path('/workspace/scratch/ui-pr269-isolated-ctl'),resources=pathlib.Path('/workspace/scratch/ui-pr269-final-resources'));g=Game(a,root,'Player')
def select(item):
 s=next(x for x in g.call('inventory')['hotbar'] if x.get('item')==item);g.call('hotbar',str(s['slot']+1));g.call('wait','5')
def hit(y):
 g.call('look-at','0.5',str(y),'2.0');g.call('wait','5');print('AIM',y,json.dumps(g.call('looking-at')),flush=True);g.call('use-item');g.call('wait','15')
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:seared_fuel_tank','give @s minecraft:lava_bucket 1','give @s minecraft:water_bucket 1','gamemode survival @s']:g.call('command',c)
 g.call('wait','10');select('minecraft:water_bucket');hit(71.4);inv=g.call('inventory');print('FILL',json.dumps(inv),flush=True);g.call('screenshot',str(root/'fill.png'));assert inv['held']['item']=='minecraft:bucket',inv
 select('minecraft:lava_bucket');hit(71.4);inv=g.call('inventory');print('REJECT',json.dumps(inv),flush=True);g.call('screenshot',str(root/'reject.png'));assert inv['held']['item']=='minecraft:lava_bucket',inv
 select('minecraft:bucket');hit(71.4);inv=g.call('inventory');print('WITHDRAW',json.dumps(inv),flush=True);g.call('screenshot',str(root/'withdraw.png'));assert inv['held']['item']=='minecraft:water_bucket',inv
 g.stop();print('PASS lower tank container fill reject withdraw',flush=True)
finally:g.cleanup()

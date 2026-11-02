import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-workstation-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-b53-tagged-lantern2');root.mkdir(exist_ok=True)
a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-client'),ctl=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/ui-pr265-resources'));g=Game(a,root,'Player')
def select(item):
 s=next(x for x in g.call('inventory')['hotbar'] if x.get('item')==item);g.call('hotbar',str(s['slot']+1));g.call('wait','5')
def hit(y):
 g.call('look-at','0.5',str(y),'2.0');g.call('wait','5');print('AIM',y,json.dumps(g.call('looking-at')),flush=True);g.call('use-item');g.call('wait','15')
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','setblock 0 71 2 tconstruct:seared_casting_tank','give @s minecraft:potion[minecraft:potion_contents={potion:"minecraft:swiftness"}] 1','give @s tconstruct:seared_lantern 1','gamemode survival @s']:g.call('command',c)
 g.call('wait','10');select('minecraft:potion');print('SOURCE',json.dumps(g.call('inventory')),flush=True);hit(71.4);inv=g.call('inventory');print('DEPOSIT',json.dumps(inv),flush=True);g.call('screenshot',str(root/'deposit.png'));assert inv['held']['item']=='minecraft:glass_bottle',inv
 select('tconstruct:seared_lantern');hit(71.4);inv=g.call('inventory');print('LANTERN_FILLED',json.dumps(inv),flush=True);g.call('screenshot',str(root/'filled.png'));data=inv['held'].get('components','');assert 'swiftness' in data,data
 hit(71.4);inv=g.call('inventory');print('LANTERN_EMPTIED',json.dumps(inv),flush=True);g.call('screenshot',str(root/'emptied.png'));assert 'swiftness' not in inv['held'].get('components',''),inv
 select('minecraft:glass_bottle');hit(71.4);inv=g.call('inventory');print('RECOVER',json.dumps(inv),flush=True);g.call('screenshot',str(root/'recovered.png'));data=inv['held'].get('components','');assert inv['held']['item']=='minecraft:potion' and 'swiftness' in data,inv
 g.stop();print('PASS tagged potion lantern roundtrip',flush=True)
finally:g.cleanup()

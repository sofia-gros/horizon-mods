import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-workstation-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-b53-casting-faucet-control');a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-client'),ctl=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/ui-b53-resources'));g=Game(a,root,'Player',resume=True)
def select(i):
 s=next(x for x in g.call('inventory')['hotbar'] if x.get('item')==i);g.call('hotbar',str(s['slot']+1));g.call('wait','5')
def empty():
 s=next(x for x in g.call('inventory')['hotbar'] if not x.get('item'));g.call('hotbar',str(s['slot']+1));g.call('wait','5')
try:
 g.ready()
 for c in ['gamemode creative @s','tick rate 20','setblock 2 70 1 minecraft:stone','tp @s 2.5 71 1.5','clear @s','setblock 0 72 1 minecraft:air','setblock 0 72 1 tconstruct:seared_faucet[facing=north]','setblock 0 71 1 minecraft:air','setblock 0 71 1 tconstruct:seared_table','give @s tconstruct:ingot_cast 1']:g.call('command',c)
 g.call('wait','10');select('tconstruct:ingot_cast');print('TABLE_USE',g.call('interact','0','71','1'),flush=True);g.call('wait','8');print('AFTER_PLACE',json.dumps(g.call('inventory')),flush=True);empty();g.call('screenshot',str(root/'placed.png'));print('FAUCET_USE',g.call('interact','0','72','1'),flush=True);g.call('wait','200',timeout=65);g.call('screenshot',str(root/'after.png'));print('TABLE_BE',json.dumps(g.call('command','data get block 0 71 1')),flush=True);print('RECOVER_USE',g.call('interact','0','71','1'),flush=True);g.call('wait','8');inv=g.call('inventory');print('RECOVER',json.dumps(inv),flush=True);assert inv['counts'].get('minecraft:copper_ingot',0)==1,inv;g.stop();print('PASS faucet ingot casting control',flush=True)
finally:g.cleanup()

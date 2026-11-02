import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-pr265-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-pr269-integrated-bacon-food');a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-pr269-isolated-client'),ctl=pathlib.Path('/workspace/scratch/ui-pr269-isolated-ctl'),resources=pathlib.Path('/workspace/scratch/ui-pr269-final-resources'));g=Game(a,root,'Player',resume=True)
try:
 g.ready()
 for c in ['gamemode creative @s','clear @s','give @s tconstruct:bacon 1','gamemode survival @s']:g.call('command',c)
 g.call('hotbar','1');g.call('look-at','2.5','85','1.5');g.call('wait','10');before=g.call('status')['player']['food'];print('BEFORE',before,flush=True);g.call('screenshot',str(root/'before.png'));assert before==10,before
 g.call('mouse','right','down');g.call('wait','45',timeout=60);g.call('wait','10');status=g.call('status');print('AFTER',status['player']['food'],json.dumps(g.call('inventory')),flush=True);g.call('screenshot',str(root/'after.png'));assert status['player']['food']==14,(before,status['player']['food']);assert not g.call('inventory')['counts'].get('tconstruct:bacon',0);g.stop();print('PASS bacon consumed and hunger increased by 4',flush=True)
finally:g.cleanup()

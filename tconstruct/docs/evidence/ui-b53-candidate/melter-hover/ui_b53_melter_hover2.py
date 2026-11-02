import sys,types,pathlib,json
sys.path.insert(0,'/workspace/scratch/ui-workstation-repo/tconstruct/tests/game');from harness import Game
root=pathlib.Path('/workspace/scratch/ui-b53-melter-hover2');a=types.SimpleNamespace(game=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-client'),ctl=pathlib.Path('/workspace/scratch/ui-b53-stage/build/fast/horizon-ctl'),resources=pathlib.Path('/workspace/scratch/ui-b53-resources'));g=Game(a,root,'Player',resume=True)
try:
 g.ready();g.call('interact','0','72','2');g.call('wait','10');ui=g.call('ui');slot=next(n for n in ui['widgets'] if n['id']=='slot-300');x,y,_,_=slot['bounds'];print('SLOT',slot,flush=True);g.call('scroll-at',str(x+8),str(y+8),'0');g.call('wait','10');g.call('screenshot',str(root/'slot-hover.png'));g.call('scroll-at',str(x+76),str(y+45),'0');g.call('wait','10');print('MENU',json.dumps(g.call('menu')),flush=True);print('UI',json.dumps(g.call('ui')),flush=True);g.call('screenshot',str(root/'hover.png'));g.call('close');g.stop()
finally:g.cleanup()

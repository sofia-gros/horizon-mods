"""Capture supported material and broken appearances in an isolated game."""
import argparse, json, tempfile, time
from pathlib import Path
from probe_horizon_server_mods import Game, ROOT
from probe_horizon_item_state import states
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--game', type=Path, default=Path('/workspace/horizon/target/fast/horizon-client'))
p.add_argument('--ctl', type=Path, default=Path('/workspace/horizon/target/fast/horizon-ctl'))
p.add_argument('--resources', type=Path, default=Path('/workspace/horizon/.local/horizon-client/26.3'))
p.add_argument('--client-mod', type=Path, default=ROOT / "Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
p.add_argument('--server-mod', type=Path, default=ROOT / "Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod")
a = p.parse_args(); a.width=1280; a.height=720
parent=ROOT/'.local/appearance-tests'; parent.mkdir(parents=True,exist_ok=True)
run=Path(tempfile.mkdtemp(prefix='visual-',dir=parent)); g=None
try:
    g=Game(a,run,'Miner'); g.ready()
    for command in ['fill -2 70 -2 2 70 3 minecraft:stone','tp @s 0.5 71 0.5','gamemode creative @s','clear @s']:
        g.call('command',command)
    variants=[('wood', 'wood', 'wood', False), ('flint','wood','wood',False), ('flint','flint','wood',False), ('flint','wood','wood',True)]
    for index,(head,handle,binding,broken) in enumerate(variants):
        state=json.dumps({'schema':1,'tool':'tconstruct:pickaxe','materials':['tconstruct:'+head,'tconstruct:'+handle,'tconstruct:'+binding],'damage':85 if broken else 0,'broken':broken},separators=(',',':'))
        g.call('command',f"item replace entity @s hotbar.{index} with minecraft:wooden_pickaxe[custom_data={{tconstruct:{{state_json:'{state}'}}}}] 1")
    g.call('wait','20'); assert len(states(g.call('inventory'))) == 4
    g.call('open-inventory'); g.call('wait','10'); g.call('screenshot',str(run/'inventory.png'),timeout=30); g.call('close')
    for index in range(4):
        g.call('select',str(index+1)); g.call('wait','10'); g.call('screenshot',str(run/f'held-{index}.png'),timeout=30)
    g.call('save'); g.stop(); g=Game(a,run,'Miner',resume=True); g.ready()
    assert len(states(g.call('inventory'))) == 4
    g.call('open-inventory'); g.call('wait','10'); g.call('screenshot',str(run/'inventory-reloaded.png'),timeout=30); g.call('close')
    g.call('select','2'); g.call('drop'); g.call('wait','10'); g.call('screenshot',str(run/'dropped.png'),timeout=30)
    print(json.dumps({'ok':True,'run':str(run)}))
finally:
    if g: g.cleanup()

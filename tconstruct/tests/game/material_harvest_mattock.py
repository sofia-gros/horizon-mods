import argparse,json
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'):p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
a=p.parse_args();run=a.run_dir.resolve();g=Game(a,run,'Player',resume=True)
def cmd(x):r=g.call('command',x);g.call('wait','6');return r
def click(s):g.call('click-slot',str(s));g.call('wait','6')
def hotbar(item):return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==item)
def mine(block):
    cmd('gamemode creative @s');cmd('tp @s 0.5 71 0.5');cmd('setblock 0 71 2 '+block);cmd('gamemode survival @s');g.call('wait','20')
    r=g.call('break','0','71','2','--timeout','40',timeout=45);print(block,json.dumps(r),flush=True);assert r.get('block')=='minecraft:air',r;return r['ticks']
try:
    g.ready()
    for x in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','give @s tconstruct:small_axe_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1','give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1','give @s tconstruct:adze_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1'):cmd(x)
    g.call('interact','0','71','2');g.call('wait','10');g.call('click-gui','-11','18');g.call('wait','5')
    for i,item in enumerate(('tconstruct:small_axe_head','tconstruct:tool_handle','tconstruct:adze_head')):click(hotbar(item));click(300+i)
    click(324);assert (g.call('menu')['carried'] or {}).get('item')=='tconstruct:mattock';click(0);g.call('close');g.call('hotbar','1')
    log=mine('minecraft:oak_log');planks=mine('minecraft:oak_planks');print('mattock log vs planks',log,planks,flush=True);assert log<planks,(log,planks)
    cmd('gamemode creative @s');cmd('item modify entity @s weapon.mainhand {type:"minecraft:set_damage",function:"minecraft:set_damage",damage:0.01}');g.call('wait','20')
    mine('minecraft:oak_log')
    held=g.call('inventory')['held'];print('broken mattock',json.dumps(held),flush=True);assert 'tic_broken:1b' in held.get('components',''),held
    broken=mine('minecraft:oak_log');print('broken vs normal',broken,log,flush=True);assert broken>log,(broken,log)
    g.call('screenshot',str(run/'material-harvest-mattock.png'));g.stop();print('PASS mattock effective blocks and broken',flush=True)
finally:g.cleanup()

import argparse, json
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'): p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
a=p.parse_args(); run=a.run_dir.resolve(); g=Game(a,run,'Player',resume=True)
def cmd(x):
    r=g.call('command',x); g.call('wait','6'); return r
def click(slot):
    g.call('click-slot',str(slot)); g.call('wait','6')
def hotbar(item):
    return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item') == item)
def build(head, handle):
    for x in ('gamemode creative @s','effect clear @s','clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:tinker_station',f'give @s tconstruct:pick_head[minecraft:custom_data={{Material:"tconstruct:{head}"}}] 1',f'give @s tconstruct:tool_handle[minecraft:custom_data={{Material:"tconstruct:{handle}"}}] 1','give @s tconstruct:tool_binding[minecraft:custom_data={Material:"tconstruct:wood"}] 1'): cmd(x)
    g.call('interact','0','71','2'); g.call('wait','10')
    for i,item in enumerate(('tconstruct:pick_head','tconstruct:tool_handle','tconstruct:tool_binding')): click(hotbar(item)); click(300+i)
    click(324); result=g.call('menu')['carried'] or {}; print('assembled',json.dumps(result),flush=True); assert result.get('item')=='tconstruct:pickaxe',result
    click(0); g.call('close'); g.call('hotbar','1'); g.call('wait','10')
def swap_head(material):
    cmd('gamemode creative @s'); cmd('tp @s 0.5 71 0.5'); cmd('setblock 0 71 2 tconstruct:tinker_station')
    cmd(f'give @s tconstruct:pick_head[minecraft:custom_data={{Material:"tconstruct:{material}"}}] 1')
    g.call('interact','0','71','2'); g.call('wait','10'); click(0); click(300); click(hotbar('tconstruct:pick_head')); click(301); click(325)
    out=g.call('menu')['carried'] or {}; print('swap',json.dumps(out),flush=True); assert out.get('item')=='tconstruct:pickaxe',out
    click(0); g.call('close'); g.call('hotbar','1'); g.call('wait','10')
def mine(ore, expected):
    for x in ('gamemode creative @s','clear @s minecraft:diamond','tp @s 0.5 71 0.5','setblock 0 71 2 '+ore): cmd(x)
    cmd('gamemode survival @s'); g.call('wait','20')
    result=g.call('break','0','71','2','--timeout','40',timeout=45); print('break',ore,json.dumps(result),flush=True); assert result.get('block')=='minecraft:air',result
    cmd('tp @s 0.5 71 2.5'); g.call('wait','60'); counts=g.call('inventory')['counts']; print('drops',json.dumps(counts),flush=True)
    assert (counts.get('minecraft:diamond',0)>0)==expected,counts
try:
    g.ready()
    if g.call('status')['screen']=='dead': g.call('respawn'); g.call('wait','20')
    build('flint','wood'); mine('minecraft:diamond_ore',False)
    cmd('gamemode creative @s');cmd('tp @s 0.5 71 0.5');cmd('setblock 0 71 2 tconstruct:tinker_station');cmd('give @s minecraft:diamond 1')
    g.call('interact','0','71','2');g.call('wait','10');click(0);click(300);click(hotbar('minecraft:diamond'));click(301);click(325)
    out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:pickaxe',out
    g.call('screenshot',str(run/'diamond-pickaxe-upgrade.png'));click(0);g.call('close');g.call('hotbar','1');g.call('wait','10')
    print('upgraded',g.call('inventory')['held'],flush=True);mine('minecraft:diamond_ore',True)
    g.call('save');g.stop();g.cleanup();g=Game(a,run,'Player',resume=True);g.ready();mine('minecraft:diamond_ore',True)
    g.stop();print('PASS diamond real drop and reload',flush=True)
finally:g.cleanup()

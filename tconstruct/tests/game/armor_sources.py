"""Material armor fire resistance and armor-bypassing damage."""
import argparse,json
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'):p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'));p.add_argument('--in-frame',action='store_true')
a=p.parse_args();g=Game(a,a.run_dir.resolve(),'Player',in_frame=a.in_frame,resume=True)
def cmd(s):r=g.call('command',s);g.call('wait','6');return r
def equip(slot,item):cmd('item replace entity @s armor.'+slot+' with '+item)
def hit(source):
    g.call('wait','35');cmd('damage @s 4 minecraft:'+source);entry=g.call('hurtlog')['players'][0]['hurts'][-1];items=g.call('inventory')['armor'];print(source,json.dumps(entry),json.dumps(items),flush=True);return entry,items
try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    for c in ('gamemode creative @s','clear @s','effect clear @s','effect give @s minecraft:instant_health 1 4 true','tp @s 20.5 100 18.5'):cmd(c)
    for slot in ('head','chest','legs','feet'):equip(slot,'minecraft:air')
    equip('head','tconstruct:plate_helmet[minecraft:max_damage=100,minecraft:damage_resistant={types:"#minecraft:is_fire"},minecraft:custom_data={tic_armor:2.0f,tic_materials:["tconstruct:iron","tconstruct:iron"],tic_max_damage:100}]')
    equip('chest','tconstruct:plate_chestplate[minecraft:max_damage=100,minecraft:custom_data={tic_armor:6.0f,tic_materials:["tconstruct:iron","tconstruct:iron"],tic_max_damage:100}]')
    cmd('gamemode survival @s');fire,after_fire=hit('lava');assert after_fire[0].get('damage',0)==0 and after_fire[1].get('damage',0)==1,after_fire
    magic,after_magic=hit('magic');assert after_magic[0].get('damage',0)==0 and after_magic[1].get('damage',0)==1,after_magic
    assert abs(magic['taken']-4.0)<0.001,magic
    g.stop();print('PASS fire resistance and armor bypass',flush=True)
finally:g.cleanup()

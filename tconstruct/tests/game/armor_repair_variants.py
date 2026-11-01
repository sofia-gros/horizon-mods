"""Material armor break timing, repair, vanilla wear and damage-source cases."""
import argparse,json
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'):p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'));p.add_argument('--in-frame',action='store_true')
a=p.parse_args();root=a.run_dir.resolve();g=Game(a,root,'Player',in_frame=a.in_frame,resume=True)
def cmd(s):
    r=g.call('command',s);g.call('wait','6');return r
def click(slot):g.call('click-slot',str(slot));g.call('wait','5')
def equip(slot,item):cmd('item replace entity @s armor.'+slot+' with '+item)
def damage(kind='minecraft:mob_attack',amount=4):
    g.call('wait','30');cmd('damage @s '+str(amount)+' '+kind);v=g.call('hurtlog')['players'][0]['hurts'][-1];print('damage',kind,json.dumps(v),flush=True);return v
def item(slot):return g.call('inventory')['armor'][slot]
head='tconstruct:plate_helmet[minecraft:damage=1,minecraft:max_damage=2,minecraft:custom_data={tic_armor:2.0f,tic_armor_toughness:4.0f,tic_materials:["tconstruct:iron","tconstruct:iron"],tic_max_damage:2}]'
chest='tconstruct:plate_chestplate[minecraft:max_damage=10,minecraft:custom_data={tic_armor:6.0f,tic_armor_toughness:0.0f,tic_materials:["tconstruct:iron","tconstruct:iron"],tic_max_damage:10}]'
try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    for kind, materials, ingredient in [('travelers_helmet','["tconstruct:iron","tconstruct:leather"]','minecraft:leather'),('slime_wings','["tconstruct:blood"]','minecraft:phantom_membrane')]:
        g.call('hotbar','1')
        for c in ('gamemode creative @s','clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:tinker_station',f'item replace entity @s weapon.mainhand with tconstruct:{kind}[minecraft:damage=10,minecraft:max_damage=100,minecraft:custom_data={{tic_max_damage:100,tic_materials:{materials}}}]','give @s '+ingredient+' 1'):cmd(c)
        g.call('interact','0','71','2');g.call('wait','10');click(0);click(300);slot=next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')==ingredient);click(slot);click(301);click(325);out=g.call('menu')['carried'] or {};print('variant repair',kind,json.dumps(out),flush=True)
        assert out.get('item')=='tconstruct:'+kind and out.get('damage',10)<10,out
        click(0);g.call('close');g.call('wait','5')
    g.stop();print('PASS armor statless and fixed repair',flush=True)
finally:g.cleanup()

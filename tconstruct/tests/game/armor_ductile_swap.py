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
    g.call('hotbar','1')
    for c in ('gamemode creative @s','clear @s','effect clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:tinker_station','item replace entity @s weapon.mainhand with tconstruct:plate_helmet[minecraft:max_damage=200,minecraft:custom_data={tic_armor:2.0f,tic_armor_toughness:0.0f,tic_materials:["tconstruct:iron","tconstruct:iron"],tic_max_damage:200}]','give @s tconstruct:helmet_plating[minecraft:custom_data={Material:"tconstruct:steel"}] 1'):cmd(c)
    g.call('interact','0','71','2');g.call('wait','10');click(0);click(300);slot=next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item')=='tconstruct:helmet_plating');click(slot);click(301);click(325);out=g.call('menu')['carried'] or {};assert out.get('item')=='tconstruct:plate_helmet',out;click(0);g.call('close');g.call('wait','10');held=g.call('inventory')['held'];print('steel replaced',json.dumps(held),flush=True)
    assert 'tconstruct:steel' in held.get('components','') and 'tic_armor_toughness:3.0f' in held.get('components',''),held
    for slot in ('head','chest','legs','feet'):equip(slot,'minecraft:air')
    g.call('interact','0','71','2');g.call('wait','10');click(0);click(500);g.call('close');g.call('wait','10')
    assert item(0).get('item')=='tconstruct:plate_helmet',item(0)
    cmd('effect give @s minecraft:instant_health 1 4 true');cmd('gamemode survival @s');entry=damage();assert entry['toughness']==3.0,entry
    g.stop();print('PASS armor Ductile material swap and equipped toughness',flush=True)
finally:g.cleanup()

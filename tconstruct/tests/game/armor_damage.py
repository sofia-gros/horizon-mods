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
    for c in ('gamemode creative @s','effect clear @s','effect give @s minecraft:instant_health 1 4 true','clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 minecraft:air','setblock 0 71 2 tconstruct:tinker_station'):
        cmd(c)
    for slot in ('head','chest','legs','feet'):equip(slot,'minecraft:air')
    cmd('gamemode survival @s');baseline=damage();assert baseline['armor']==0,baseline
    cmd('gamemode creative @s');cmd('effect give @s minecraft:instant_health 1 4 true');equip('head',head);equip('chest',chest);cmd('gamemode survival @s')
    first=damage();helmet=item(0);body=item(1);print('first armor',json.dumps(helmet),json.dumps(body),flush=True)
    assert first['armor']==8 and first['toughness']==4 and first['taken']<baseline['taken'],(first,baseline)
    assert helmet.get('item')=='tconstruct:plate_helmet' and helmet.get('damage')==2 and 'tic_broken:1b' in helmet.get('components',''),helmet
    assert body.get('damage')==1,body
    second=damage();helmet=item(0);body=item(1);assert second['armor']==6 and second['toughness']==0 and helmet.get('damage')==2 and body.get('damage')==2,(second,helmet,body)
    # Use the Station's equipment slots to move the retained helmet into repair.
    cmd('gamemode creative @s');cmd('give @s minecraft:iron_ingot 1');g.call('interact','0','71','2');g.call('wait','10');click(500);click(300)
    slot=next(x['slot'] for x in g.call('inventory')['hotbar'] if x.get('item')=='minecraft:iron_ingot');click(slot);click(301);click(325)
    repaired=g.call('menu')['carried'] or {};print('repair result',json.dumps(repaired),flush=True);assert repaired.get('item')=='tconstruct:plate_helmet' and repaired.get('damage',2)<2,repaired
    g.call('screenshot',str(root/'armor-station-repair.png'));click(500);g.call('close');g.call('wait','10');restored=item(0);print('restored',json.dumps(restored),flush=True);assert restored.get('item')=='tconstruct:plate_helmet' and 'tic_broken:0b' in restored.get('components',''),restored
    cmd('effect give @s minecraft:instant_health 1 4 true');cmd('gamemode survival @s');third=damage();assert third['armor']==8 and third['toughness']==4,third
    g.call('screenshot',str(root/'armor-repair.png'));g.call('save');g.stop();g.cleanup();g=Game(a,root,'Player',in_frame=a.in_frame,resume=True);g.ready();saved=item(0);print('saved',json.dumps(saved),flush=True);assert saved.get('item')=='tconstruct:plate_helmet',saved
    g.stop();print('PASS material armor break repair and save',flush=True)
finally:g.cleanup()

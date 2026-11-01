"""Hosted guest armor wear and break synchronization."""
import argparse,json,time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for k in ('game','ctl','resources'):p.add_argument('--'+k,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
a=p.parse_args();root=a.run_dir.resolve();games=[]
def cmd(g,s):r=g.call('command',s);g.call('wait','6');return r
def wait(fn):
    for _ in range(100):
        if fn():return
        time.sleep(.2)
    raise TimeoutError('LAN armor state')
try:
    h=Game(a,root,'Player',resume=True);games.append(h);h.ready();port=h.call('lan','open','0','commands')['port']
    q=Game(a,root,'Guest',server='127.0.0.1:'+str(port),resume=(root/'Guest').exists());games.append(q);q.ready()
    cmd(h,'gamemode creative @a');cmd(q,'effect give @s minecraft:instant_health 1 4 true')
    for slot in ('head','chest','legs','feet'):cmd(q,'item replace entity @s armor.'+slot+' with minecraft:air')
    gear='tconstruct:plate_helmet[minecraft:damage=1,minecraft:max_damage=2,minecraft:custom_data={tic_armor:2.0f,tic_armor_toughness:4.0f,tic_materials:["tconstruct:iron","tconstruct:iron"],tic_max_damage:2}]'
    cmd(q,'item replace entity @s armor.head with '+gear)
    wait(lambda:q.call('inventory')['armor'][0].get('item')=='tconstruct:plate_helmet')
    cmd(h,'gamemode survival Guest');q.call('wait','30');h.call('hurtlog');print('guest damage command',cmd(q,'damage @s 4 minecraft:mob_attack'),flush=True)
    wait(lambda:q.call('inventory')['armor'][0].get('damage')==2)
    first=h.call('hurtlog');gear=q.call('inventory')['armor'][0];print('guest first',json.dumps(first),json.dumps(gear),flush=True)
    entries=[p for p in first['players'] if p['name']=='Guest'];assert entries and entries[0]['hurts'][-1]['armor']==2 and entries[0]['hurts'][-1]['toughness']==4,first
    assert 'tic_broken:1b' in gear.get('components',''),gear
    q.call('wait','35');cmd(q,'damage @s 4 minecraft:mob_attack');second=h.call('hurtlog');print('guest second',json.dumps(second),flush=True)
    entries=[p for p in second['players'] if p['name']=='Guest'];assert entries and entries[0]['hurts'][-1]['armor']==0 and entries[0]['hurts'][-1]['toughness']==0,second
    q.call('screenshot',str(root/'armor-damage-lan.png'));q.stop();q.cleanup();games.remove(q);h.call('save');h.stop();print('PASS LAN armor break and attributes',flush=True)
finally:
    for g in reversed(games):g.cleanup()

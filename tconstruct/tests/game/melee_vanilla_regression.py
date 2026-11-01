"""Vanilla melee remains on the engine's existing damage/wear path."""
import argparse
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for key in ('game','ctl','resources'):p.add_argument('--'+key,type=Path,required=True)
p.add_argument('--run-dir',type=Path,default=Path('target/pb-station-batch'))
a=p.parse_args();g=Game(a,a.run_dir.resolve(),'Player',resume=True)
def cmd(s):g.call('command',s);g.call('wait','5')
try:
    g.ready()
    if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
    for command in ('gamemode creative @s','clear @s','kill @e[type=minecraft:husk,tag=source_melee_test]',
                    'fill 17 99 17 23 99 23 minecraft:stone','fill 17 100 17 23 104 23 minecraft:air',
                    'tp @s 20.5 100 18.5','give @s minecraft:iron_sword 1',
                    'summon minecraft:husk 20.5 100 20.5 {NoAI:1b,Tags:["source_melee_test"]}',
                    'gamemode survival @s'):cmd(command)
    g.call('hotbar','1');g.call('wait','40');before=g.call('inventory')['held'];target=g.call('entities','10','minecraft:husk')['entities'][0]
    result=g.call('attack',str(target['id']));g.call('wait','8');after=g.call('inventory')['held']
    assert result['health']<result['health_before'],result
    assert after['item']=='minecraft:iron_sword' and after.get('damage',0)-before.get('damage',0)==1,(before,after)
    print('PASS vanilla iron sword primary damage and wear',flush=True);g.stop()
finally:g.cleanup()

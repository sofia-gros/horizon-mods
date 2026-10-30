import argparse
import json
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)

def cmd(command):
    result = g.call('command', command)
    g.call('wait', '6')
    return result

def arrows():
    return g.call('inventory')['counts'].get('minecraft:arrow', 0)

def setup(item):
    for x in ('gamemode creative @s', 'clear @s', 'effect clear @s', 'effect give @s minecraft:instant_health 1 4 true', 'kill @e[type=minecraft:zombie]', 'tp @s 0.5 71 0.5', 'item replace entity @s weapon.mainhand with ' + item, 'give @s minecraft:arrow 8', 'gamemode survival @s'):
        cmd(x)
    g.call('wait', '15')
try:
    g.ready()
    if g.call('status')['screen'] == 'dead':
        g.call('respawn')
        g.call('wait', '20')
    for item in ['minecraft:bow', 'tconstruct:longbow']:
        setup(item)
        before = arrows()
        g.call('mouse', 'right', 'down')
        g.call('wait', '30')
        status = g.call('status')
        print(item, 'draw', status['using'], flush=True)
        assert status['using'] is not None
        if item == 'tconstruct:longbow':
            g.call('screenshot', str(run / 'equipment-longbow-draw.png'))
        g.call('mouse', 'right', 'up')
        g.call('wait', '20')
        after = arrows()
        print(item, 'arrows', before, after, flush=True)
        assert after == before - 1
    setup('tconstruct:longbow')
    before = arrows()
    g.call('mouse', 'right', 'down')
    g.call('wait', '8')
    assert g.call('status')['using'] is not None
    g.call('hotbar', '2')
    g.call('wait', '8')
    g.call('mouse', 'right', 'up')
    g.call('wait', '15')
    after = arrows()
    print('interrupted arrows', before, after, flush=True)
    assert before == after
    for x in ('gamemode creative @s', 'effect clear @s', 'kill @e[type=minecraft:zombie]', 'tp @s 0.5 71 0.5', 'item replace entity @s weapon.mainhand with tconstruct:pickaxe[minecraft:custom_data={tic_attack_damage:1.75f,tic_attack_speed:1.2f,tic_max_damage:85},minecraft:max_damage=85]', 'effect give @s minecraft:weakness 30 0 true', 'gamemode survival @s', 'summon minecraft:zombie 1.5 71 0.5 {NoAI:1b}'):
        cmd(x)
    g.call('wait', '30')
    g.call('hurtlog')
    ids = g.call('entities', '8', 'minecraft:zombie')['entities']
    result = g.call('attack', str(ids[0]['id']))
    print('weakness attack', json.dumps(result), json.dumps(g.call('hurtlog')), flush=True)
    assert result['health'] <= result['health_before']
    print('PASS', flush=True)
    g.stop()
finally:
    g.cleanup()

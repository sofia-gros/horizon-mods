import argparse, json
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
for arg in ('game', 'ctl', 'resources'):
    p.add_argument('--' + arg, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)

def cmd(x):
    out = g.call('command', x)
    g.call('wait', '6')
    return out

def arrows():
    return g.call('inventory')['counts'].get('minecraft:arrow', 0)

def setup(item):
    for x in ('gamemode creative @s', 'effect clear @s', 'effect give @s minecraft:instant_health 1 4 true', 'kill @e[type=minecraft:zombie]', 'clear @s', 'tp @s 0.5 71 0.5', 'item replace entity @s weapon.mainhand with ' + item, 'give @s minecraft:arrow 8', 'gamemode survival @s'):
        cmd(x)
    g.call('wait', '10')

def bow(speed):
    return 'tconstruct:longbow[minecraft:custom_data={tic_draw_speed:' + str(speed) + 'f,tic_velocity:1.0f,tic_accuracy:1.0f,tic_max_damage:100},minecraft:max_damage=100]'

def cross(speed):
    return 'tconstruct:crossbow[minecraft:custom_data={tic_draw_speed:' + str(speed) + 'f,tic_velocity:1.0f,tic_accuracy:1.0f,tic_max_damage:100},minecraft:max_damage=100]'

def shoot_bow(item, wait):
    setup(item)
    before = arrows()
    g.call('mouse', 'right', 'down')
    g.call('wait', str(wait))
    print('bow draw', item, g.call('status')['using'], flush=True)
    g.call('mouse', 'right', 'up')
    g.call('wait', '10')
    after = arrows()
    print('bow arrows', before, after, flush=True)
    return before - after
try:
    g.ready()
    if g.call('status')['screen'] == 'dead':
        g.call('respawn')
        g.call('wait', '20')
    assert shoot_bow('minecraft:bow', 30) == 1
    assert shoot_bow(bow(2.0), 4) == 1
    assert shoot_bow(bow(0.25), 4) == 0
    for name, item in [('vanilla', 'minecraft:crossbow'), ('tconstruct', cross(2.0))]:
        setup(item)
        before = arrows()
        g.call('mouse', 'right', 'down')
        g.call('wait', '45')
        during = g.call('status')
        print(name, 'charge', during['using'], 'arrows', before, arrows(), flush=True)
        assert arrows() == before - 1
        if name == 'tconstruct':
            g.call('screenshot', str(run / 'equipment-crossbow-charge.png'))
        g.call('mouse', 'right', 'up')
        g.call('wait', '8')
        before_fire = g.call('status')
        g.call('mouse', 'right', 'down')
        g.call('wait', '2')
        g.call('mouse', 'right', 'up')
        g.call('wait', '12')
        after_fire = g.call('status')
        print(name, 'fire counts', before_fire['server']['entities'].get('minecraft:arrow', 0), after_fire['server']['entities'].get('minecraft:arrow', 0), flush=True)
        assert after_fire['server']['entities'].get('minecraft:arrow', 0) > before_fire['server']['entities'].get('minecraft:arrow', 0)
    setup(cross(0.5))
    before = arrows()
    g.call('mouse', 'right', 'down')
    g.call('wait', '8')
    g.call('hotbar', '2')
    g.call('wait', '5')
    g.call('mouse', 'right', 'up')
    g.call('wait', '10')
    print('interrupted', before, arrows(), flush=True)
    assert before == arrows()
    for speed in (2.0, 0.5):
        setup(cross(speed))
        before = arrows()
        g.call('mouse', 'right', 'down')
        g.call('wait', '16')
        after = arrows()
        print('charge speed', speed, 'arrows', before, after, flush=True)
        assert after == before - (1 if speed == 2.0 else 0)
        g.call('mouse', 'right', 'up')
        g.call('wait', '6')
    print('PASS ranged', flush=True)
    g.stop()
finally:
    g.cleanup()

"""Exercise representative registered blocks in the reusable LAN world."""
import argparse
import time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for name in ('game', 'ctl', 'resources'):
    p.add_argument('--' + name, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
games = []

def wait(label, predicate):
    for _ in range(120):
        if predicate(): return
        time.sleep(.2)
    raise AssertionError(label)

def command(game, text):
    game.call('command', text)
    game.call('wait', '8')

try:
    host = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); games.append(host); host.ready()
    port = host.call('lan', 'open', '0', 'commands')['port']
    guest = Game(a, run, 'Guest', server='127.0.0.1:' + str(port), resume=True); games.append(guest); guest.ready()
    for text in ('gamemode creative @a', 'kill @e[type=minecraft:zombie,tag=tconstruct_upgrade_test]',
                 'fill -2 70 -1 4 70 5 minecraft:stone', 'tp @s -0.5 71 0.5', 'tp Guest 1.5 71 0.5', 'clear @a'):
        command(host, text)
    samples = ('grout', 'seared_bricks_slab', 'greenheart_planks_stairs', 'greenheart_door',
               'scorched_bricks_fence', 'cobalt_block', 'slimesteel_block', 'earth_congealed_slime',
               'crafting_station', 'modifier_worktable')
    for name in samples:
        command(host, 'setblock 0 71 2 minecraft:air')
        command(host, 'clear @s')
        command(host, f'give @s tconstruct:{name} 1')
        host.call('place', '0', '71', '2', '--item', f'tconstruct:{name}')
        wait('host placed ' + name, lambda: host.call('block', '0', '71', '2').get('block') == 'tconstruct:' + name)
        wait('guest saw ' + name, lambda: guest.call('block', '0', '71', '2').get('block') == 'tconstruct:' + name)
        if name in ('slimesteel_block', 'earth_congealed_slime', 'crafting_station', 'modifier_worktable'):
            host.call('look-at', 'block', '0', '71', '2'); host.call('wait', '40')
            host.call('screenshot', str(run / ('block-registration-' + name + '.png')))
    # Save a placed source block, then reopen the same world.
    command(host, 'setblock 0 71 2 tconstruct:seared_bricks')
    wait('guest saw saved block', lambda: guest.call('block', '0', '71', '2').get('block') == 'tconstruct:seared_bricks')
    guest.stop(); host.call('save'); host.stop(); guest.cleanup(); host.cleanup(); games.clear()
    host = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); games.append(host); host.ready()
    assert host.call('block', '0', '71', '2')['block'] == 'tconstruct:seared_bricks'
    for text in ('gamemode survival @s', 'clear @s', 'give @s minecraft:diamond_pickaxe 1'):
        command(host, text)
    host.call('select', 'minecraft:diamond_pickaxe'); host.call('wait', '10')
    host.call('break', '0', '71', '2', '--timeout', '30', timeout=35)
    command(host, 'tp @s 0.5 71 2.5'); host.call('wait', '60')
    wait('seared brick drop', lambda: any(s.get('item') == 'tconstruct:seared_bricks' for s in host.call('inventory')['hotbar']))
    host.stop()
    print('PASS representative placement, LAN, save and mining', flush=True)
finally:
    for game in reversed(games): game.cleanup()

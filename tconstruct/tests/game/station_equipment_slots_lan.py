"""Exercise workstation tabs and carried-item preservation over LAN."""
import argparse
import time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for key in ('game', 'ctl', 'resources'):
    p.add_argument('--' + key, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
a = p.parse_args()
run = a.run_dir.resolve()
games = []

def wait(label, test):
    for _ in range(120):
        if test(): return
        time.sleep(.2)
    raise AssertionError(label)

def widget(game, slot):
    return any(row.get('id') == f'slot-{slot}' for row in game.call('ui')['widgets'])

try:
    host = Game(a, run, 'Player', resume=True); games.append(host); host.ready()
    port = host.call('lan', 'open', '0', 'commands')['port']
    guest = Game(a, run, 'Guest', server='127.0.0.1:' + str(port), resume=True); games.append(guest); guest.ready()
    for command in ('gamemode creative @a', 'clear @a', 'tp @s -2.5 71 0.5', 'tp Guest 0.5 71 0.5',
                    'setblock -1 71 1 minecraft:air', 'setblock 0 71 1 minecraft:air', 'setblock 1 71 1 minecraft:air',
                    'setblock 0 71 2 minecraft:air', 'setblock 1 71 2 minecraft:air',
                    'setblock 0 71 2 tconstruct:part_builder', 'setblock 1 71 2 tconstruct:tinker_station',
                    'give Guest minecraft:stone 1', 'give Guest minecraft:iron_helmet 1'):
        host.call('command', command)
    host.call('wait', '15'); guest.call('interact', '0', '71', '2'); guest.call('wait', '10')
    wait('Part Builder', lambda: widget(guest, 322))
    guest.call('click-gui', '45', '-15'); guest.call('wait', '10')
    wait('Station', lambda: widget(guest, 500))
    hotbar = next(slot['slot'] for slot in guest.call('inventory')['hotbar'] if slot.get('item') == 'minecraft:iron_helmet')
    guest.call('click-slot', str(hotbar)); guest.call('wait', '5'); guest.call('click-slot', '500'); guest.call('wait', '10')
    wait('helmet equipped', lambda: guest.call('inventory')['armor'][0].get('item') == 'minecraft:iron_helmet')
    guest.call('click-slot', '500', 'shift'); guest.call('wait', '10')
    wait('helmet unequipped', lambda: not guest.call('inventory')['armor'][0].get('item'))
    hotbar = next(slot['slot'] for slot in guest.call('inventory')['hotbar'] if slot.get('item') == 'minecraft:stone')
    guest.call('click-slot', str(hotbar)); guest.call('wait', '5'); guest.call('click-slot', '504'); guest.call('wait', '10')
    wait('offhand equipped', lambda: any(row.get('id') == 'slot-504' and (row.get('stack') or {}).get('item') == 'minecraft:stone' for row in guest.call('ui')['widgets']))
    guest.call('click-slot', '504'); guest.call('wait', '10')
    assert (guest.call('menu').get('carried') or {}).get('item') == 'minecraft:stone'
    guest.call('close'); host.call('save')
    for game in reversed(games): game.stop()
    print('PASS LAN Station armor and offhand slots', flush=True)

finally:
    for game in reversed(games): game.cleanup()

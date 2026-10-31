"""Check Melter inputs and one cast across the reusable LAN test world."""
import argparse
import time
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
run.mkdir(parents=True, exist_ok=True)
games = []


def wait(label, predicate):
    for _ in range(120):
        if predicate(): return
        time.sleep(.2)
    raise AssertionError(label)


def hotbar(game, item):
    return next(slot['slot'] for slot in game.call('inventory')['hotbar'] if slot.get('item') == item)


def click(game, slot):
    game.call('click-slot', str(slot)); game.call('wait', '5')


def slots(game):
    return {row['slot']: row['stack'] for row in game.call('menu')['slots'] if row.get('stack')}


try:
    host = Game(a, run, 'Player', in_frame=a.in_frame, resume=(run / 'Player').exists()); games.append(host); host.ready()
    port = host.call('lan', 'open', '0', 'commands')['port']
    guest = Game(a, run, 'Guest', server='127.0.0.1:' + str(port), resume=(run / 'Guest').exists()); games.append(guest); guest.ready()
    for command in ('gamemode creative @a', 'clear @a', 'tp Guest 2.5 71 2.5',
                    'setblock 0 72 2 minecraft:air', 'setblock 0 71 2 minecraft:air',
                    'setblock 0 71 2 tconstruct:seared_fuel_tank', 'setblock 0 72 2 tconstruct:seared_melter',
                    'give Guest minecraft:lava_bucket 1', 'give Guest tconstruct:molten_copper_bucket 1',
                    'give Guest minecraft:copper_ingot 1'):
        host.call('command', command)
    host.call('wait', '20'); guest.call('interact', '0', '72', '2'); guest.call('wait', '15')
    click(guest, hotbar(guest, 'minecraft:lava_bucket'))
    guest.call('click-gui', '158', '50', 'right'); guest.call('wait', '10')
    wait('empty fuel bucket', lambda: (guest.call('menu').get('carried') or {}).get('item') == 'minecraft:bucket')
    guest.call('click-gui', '158', '50'); guest.call('wait', '10')
    wait('filled fuel bucket', lambda: (guest.call('menu').get('carried') or {}).get('item') == 'minecraft:lava_bucket')
    guest.call('click-gui', '158', '50', 'right'); guest.call('wait', '10')
    wait('returned fuel bucket', lambda: (guest.call('menu').get('carried') or {}).get('item') == 'minecraft:bucket')
    empty = next(slot['slot'] for slot in guest.call('inventory')['hotbar'] if not slot.get('item'))
    click(guest, empty); click(guest, hotbar(guest, 'tconstruct:molten_copper_bucket'))
    guest.call('click-gui', '100', '45', 'right'); guest.call('wait', '10')
    wait('empty output bucket', lambda: (guest.call('menu').get('carried') or {}).get('item') == 'minecraft:bucket')
    guest.call('click-gui', '100', '45'); guest.call('wait', '10')
    wait('filled output bucket', lambda: (guest.call('menu').get('carried') or {}).get('item') == 'tconstruct:molten_copper_bucket')
    empty = next(slot['slot'] for slot in guest.call('inventory')['hotbar'] if not slot.get('item'))
    click(guest, empty); click(guest, hotbar(guest, 'minecraft:copper_ingot')); click(guest, 300)
    host.call('wait', '500', timeout=55)
    wait('melted copper', lambda: 300 not in slots(guest))
    guest.call('screenshot', str(run / 'melter-fluid-fuel-lan.png'))
    guest.call('close'); host.call('save')
    for game in reversed(games): game.stop()
    print('PASS LAN Melter tank clicks and liquid-fuel melting', flush=True)

finally:
    for game in reversed(games): game.cleanup()

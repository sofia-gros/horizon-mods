"""Exercise per-viewer Smeltery bucket transfers over LAN."""
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
        if predicate():
            return
        time.sleep(.2)
    raise AssertionError(label)


def hotbar(game, item):
    return next(slot['slot'] for slot in game.call('inventory')['hotbar'] if slot.get('item') == item)


def select(game, item):
    game.call('hotbar', str(hotbar(game, item) + 1))
    game.call('wait', '10')


def click(game, slot):
    game.call('click-slot', str(slot))
    game.call('wait', '5')


def slots(game):
    return {int(row['id'][5:]): row['stack'] for row in game.call('ui')['widgets']
            if row.get('id', '').startswith('slot-') and row.get('stack')}


try:
    host = Game(a, run, 'Player', in_frame=a.in_frame, resume=(run / 'Player').exists())
    games.append(host)
    host.ready()
    port = host.call('lan', 'open', '0', 'commands')['port']
    guest = Game(a, run, 'Guest', server='127.0.0.1:' + str(port), resume=(run / 'Guest').exists())
    games.append(guest)
    guest.ready()
    for x, y, z in ((0,71,1), (1,71,2), (0,71,3), (0,71,4), (0,70,4)):
        host.call('command', f'setblock {x} {y} {z} minecraft:air')
    host.call('wait', '10')
    for command in (
        'gamemode creative @a', 'clear @a', 'tp @s 2.5 71 1.5', 'tp Guest 3.5 71 1.5',
        'setblock 0 70 2 tconstruct:seared_bricks', 'setblock 0 71 2 minecraft:air',
        'setblock -1 71 2 tconstruct:seared_bricks', 'setblock 1 71 2 tconstruct:seared_fuel_tank',
        'setblock 0 71 3 tconstruct:seared_drain[facing=south]',
        'setblock 0 71 1 tconstruct:smeltery_controller[facing=north,lit=false]',
        'setblock 0 71 4 tconstruct:seared_faucet[facing=south]', 'setblock 0 70 4 tconstruct:seared_table',
        'give Guest minecraft:lava_bucket 1', 'give Guest tconstruct:molten_copper_bucket 1', 'give Guest minecraft:copper_ingot 1',
        'give Guest minecraft:amethyst_shard 1', 'give Guest tconstruct:ingot_cast 1',
    ):
        host.call('command', command)
    host.call('wait', '20')
    select(guest, 'minecraft:lava_bucket')
    guest.call('interact', '1', '71', '2')
    guest.call('wait', '10')
    host.call('command', 'tp @s 1.5 71 -1.5')
    host.call('command', 'tp Guest -1.5 71 -1.5')
    host.call('wait', '10')
    for game in games:
        empty = next(slot['slot'] for slot in game.call('inventory')['hotbar'] if not slot.get('item'))
        game.call('hotbar', str(empty + 1))
        game.call('wait', '10')
        game.call('interact', '0', '71', '1')
        game.call('wait', '10')
        assert any(w.get('id') == 'slot-302' for w in game.call('ui')['widgets']), game.label + ' input missing'
    click(guest, hotbar(guest, 'tconstruct:molten_copper_bucket'))
    click(guest, 300)
    wait('guest empty bucket', lambda: slots(guest).get(301, {}).get('item') == 'minecraft:bucket')
    assert 301 not in slots(host), 'guest container leaked into host menu'
    click(guest, 301); click(guest, 300)
    wait('guest filled bucket', lambda: slots(guest).get(301, {}).get('item') == 'tconstruct:molten_copper_bucket')
    guest.call('close'); guest.call('wait', '10')
    wait('returned on close', lambda: guest.call('inventory')['counts'].get('tconstruct:molten_copper_bucket', 0) == 1)
    host.call('close'); host.call('save')
    for game in reversed(games):
        game.stop()
    print('PASS LAN container transfer, per-viewer isolation and close return', flush=True)

finally:
    for game in reversed(games):
        game.cleanup()

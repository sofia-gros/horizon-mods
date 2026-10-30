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
    for command in (
        'gamemode creative @a', 'clear @a', 'tp @s -0.5 71 0.5', 'tp Guest 1.5 71 0.5',
        'setblock 0 71 2 minecraft:air', 'setblock 0 70 2 minecraft:air',
        'setblock 0 71 1 minecraft:air', 'setblock 0 70 1 minecraft:air',
        'setblock 0 70 2 tconstruct:seared_heater', 'setblock 0 71 2 tconstruct:seared_melter',
        'setblock 0 71 1 tconstruct:seared_faucet[facing=north]', 'setblock 0 70 1 tconstruct:seared_table',
        'give Guest minecraft:coal 1', 'give Guest minecraft:copper_ingot 1', 'give Guest tconstruct:ingot_cast 1',
    ):
        host.call('command', command)
    host.call('interact', '0', '70', '2'); host.call('wait', '5'); host.call('close')
    for game in games:
        wait(game.label + ' melter', lambda game=game: game.call('block', '0', '71', '2').get('block') == 'tconstruct:seared_melter')
        game.call('interact', '0', '71', '2'); game.call('wait', '10')
    click(guest, hotbar(guest, 'minecraft:coal')); click(guest, 327)
    click(guest, hotbar(guest, 'minecraft:copper_ingot')); click(guest, 300)
    wait('shared input', lambda: all(slots(game).get(300, {}).get('item') == 'minecraft:copper_ingot' for game in games))
    host.call('wait', '400', timeout=45)
    wait('melted input', lambda: all(300 not in slots(game) for game in games))
    guest.call('screenshot', str(run / 'melter-lan.png'))
    for game in games: game.call('close')
    guest.call('hotbar', str(hotbar(guest, 'tconstruct:ingot_cast') + 1)); guest.call('interact', '0', '70', '1')
    empty = next(slot['slot'] for slot in guest.call('inventory')['hotbar'] if not slot.get('item'))
    guest.call('hotbar', str(empty + 1)); guest.call('interact', '0', '71', '1'); guest.call('wait', '120', timeout=45)
    guest.call('interact', '0', '70', '1'); guest.call('wait', '5')
    assert guest.call('inventory')['counts'].get('minecraft:copper_ingot', 0) == 1
    for game in reversed(games): game.stop()
    print('PASS LAN shared Melter fuel, melting and guest casting', flush=True)
finally:
    for game in reversed(games): game.cleanup()

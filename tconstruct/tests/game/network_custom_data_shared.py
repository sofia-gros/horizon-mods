"""Two-player custom_data synchronization through a shared vanilla chest."""
import argparse
import tempfile
import time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
root = Path(tempfile.mkdtemp(prefix='network-custom-shared-', dir=Path('target').resolve()))
print('run', root, flush=True)
games = []

def wait(label, predicate):
    for _ in range(100):
        if predicate():
            print('ok', label, flush=True); return
        time.sleep(.2)
    raise AssertionError(label)

def proof(stack):
    return bool(stack and stack.get('item') == 'minecraft:diamond'
                and 'proof:17' in str(stack.get('components', '')))

def slot(game, index):
    return next((s for s in game.call('screen').get('slots', []) if s['slot'] == index and s.get('item')), None)

def click(game, index):
    game.call('click-slot', str(index)); game.call('wait', '3')

try:
    host = Game(a, root, 'Host', in_frame=a.in_frame); games.append(host); host.ready()
    port = host.call('lan', 'open', '0', 'commands')['port']
    guest = Game(a, root, 'Guest', server='127.0.0.1:' + str(port)); games.append(guest); guest.ready()
    for command in ('fill -2 70 -1 4 70 5 minecraft:stone', 'gamemode creative @a',
                    'tp @s -0.5 71 0.5', 'tp Guest 1.5 71 0.5',
                    'setblock 0 71 2 minecraft:chest', 'clear @a',
                    'give @s minecraft:diamond[minecraft:custom_data={proof:17}] 1'):
        host.call('command', command)
    wait('host inventory', lambda: any(proof(s) for s in host.call('inventory')['hotbar']))
    for game in games:
        wait(game.label + ' chest', lambda: game.call('block', '0', '71', '2').get('block') == 'minecraft:chest')
        game.call('interact', '0', '71', '2')
        wait(game.label + ' menu', lambda: any(w['id'] == 'slot-300' for w in game.call('ui')['widgets']))
    index = next(s['slot'] for s in host.call('inventory')['hotbar'] if proof(s))
    click(host, index); click(host, 300)
    wait('guest sees proof', lambda: proof(slot(guest, 300)))
    click(guest, 300); click(guest, 0); guest.call('close')
    wait('guest receives proof', lambda: any(proof(s) for s in guest.call('inventory')['hotbar']))
    wait('host sees empty', lambda: slot(host, 300) is None)
    guest.call('interact', '0', '71', '2')
    wait('guest reopened', lambda: any(w['id'] == 'slot-300' for w in guest.call('ui')['widgets']))
    index = next(s['slot'] for s in guest.call('inventory')['hotbar'] if proof(s))
    click(guest, index); click(guest, 300)
    wait('host sees returned proof', lambda: proof(slot(host, 300)))
    guest.call('close'); guest.stop(); guest.cleanup(); games.remove(guest)
    host.call('close'); host.stop(); host.cleanup(); games.remove(host)
    host = Game(a, root, 'Host', in_frame=a.in_frame, resume=True); games.append(host); host.ready()
    host.call('interact', '0', '71', '2')
    wait('saved shared chest proof', lambda: proof(slot(host, 300)))
    host.stop(); host.cleanup(); games.remove(host)
finally:
    for game in reversed(games): game.cleanup()

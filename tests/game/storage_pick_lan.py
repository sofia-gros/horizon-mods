"""Exercise material Pick Block by a connected LAN guest."""
import argparse
import json
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
run = Path(tempfile.mkdtemp(prefix='storage-pick-lan-', dir=Path('target').resolve()))
print('run', run, flush=True)
games = []

def wait(label, predicate):
    for _ in range(150):
        if predicate():
            print('ok', label, flush=True)
            return
        time.sleep(.2)
    raise AssertionError(label)

try:
    host = Game(a, run, 'Host', in_frame=a.in_frame); games.append(host); host.ready()
    pack = host.root / 'saves/Probe/datapacks/storage-test'
    definition = pack / 'data/tconstruct/tinkering/materials/definition/bronze.json'
    definition.parent.mkdir(parents=True)
    definition.write_text(json.dumps({'craftable': True}))
    (pack / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"storage pick test"}}')
    host.call('command', 'reload'); host.call('wait', '40')
    port = host.call('lan', 'open', '0', 'commands')['port']
    guest = Game(a, run, 'Guest', server='127.0.0.1:' + str(port)); games.append(guest); guest.ready()
    for cmd in ('fill -2 70 -1 4 70 5 minecraft:stone', 'gamemode creative @a', 'tp @s -0.5 71 0.5',
                'tp Guest 1.5 71 0.5', 'clear @s',
                'give @s tconstruct:fake_storage_block[minecraft:custom_data={Material:"tconstruct:bronze"}] 1'):
        host.call('command', cmd)
    host.call('wait', '30'); host.call('place', '0', '71', '2', '--item', 'tconstruct:fake_storage_block')
    wait('guest block', lambda: guest.call('block', '0', '71', '2').get('block') == 'tconstruct:fake_storage_block')
    guest.call('wait', '40'); guest.call('look-at', 'block', '0', '71', '2')
    for ctrl in (False, True):
        host.call('command', 'clear Guest'); guest.call('wait', '10')
        if ctrl: guest.call('key', 'ctrl', 'down')
        guest.call('mouse', 'middle', 'click')
        if ctrl: guest.call('key', 'ctrl', 'up')
        wait('guest material pick ctrl=' + str(ctrl), lambda: any(s.get('item') == 'tconstruct:fake_storage_block' and 'tconstruct:bronze' in s.get('components', '') for s in guest.call('inventory')['hotbar']))
    guest.call('screenshot', str(run / 'guest-pick.png'))
    host.call('command', 'clear Guest'); host.call('command', 'setblock 1 71 2 minecraft:stone')
    wait('guest stone', lambda: guest.call('block', '1', '71', '2').get('block') == 'minecraft:stone')
    guest.call('look-at', 'block', '1', '71', '2'); guest.call('mouse', 'middle', 'click')
    wait('ordinary stone pick', lambda: any(s.get('item') == 'minecraft:stone' for s in guest.call('inventory')['hotbar']))
    for game in reversed(games): game.stop()
    print('PASS LAN material and ordinary Pick Block', flush=True)
finally:
    for game in reversed(games): game.cleanup()

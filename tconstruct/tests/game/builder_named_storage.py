"""Show a genuinely named adjacent chest in the existing test world."""
import argparse
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for key in ('game', 'ctl', 'resources'): p.add_argument('--' + key, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--lan', action='store_true')
a = p.parse_args(); run = a.run_dir.resolve(); games = []
try:
    host = Game(a, run, 'Player', resume=True); games.append(host); host.ready()
    for command in ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5',
                    'setblock 0 71 1 minecraft:air', 'setblock 0 71 2 minecraft:air', 'setblock 1 71 2 minecraft:air',
                    'setblock 0 71 2 tconstruct:part_builder',
                    '''give @s minecraft:chest[minecraft:custom_name='"Parts Cache"'] 1'''):
        host.call('command', command)
    host.call('wait', '15'); host.call('place', '1', '71', '2', '--item', 'minecraft:chest'); host.call('wait', '15')
    host.call('save')
    if a.lan:
        port = host.call('lan', 'open', '0', 'commands')['port']
        guest = Game(a, run, 'Guest', server='127.0.0.1:' + str(port), resume=True); games.append(guest); guest.ready()
        host.call('command', 'tp @s -2.5 71 0.5'); host.call('command', 'tp Guest 0.5 71 0.5'); actor = guest
    else: actor = host
    actor.call('wait', '15'); actor.call('interact', '0', '71', '2'); actor.call('wait', '30')
    assert any(w.get('id') == 'slot-327' for w in actor.call('ui')['widgets']), 'side inventory missing'
    actor.call('wait', '220')
    actor.call('screenshot', str(run / ('builder-name-lan.png' if a.lan else 'builder-name.png')))
    actor.call('close'); host.call('save')
    for game in reversed(games): game.stop()
    print('PASS named chest placed and adjacent inventory opened; inspect screenshot for title', flush=True)
finally:
    for game in reversed(games): game.cleanup()

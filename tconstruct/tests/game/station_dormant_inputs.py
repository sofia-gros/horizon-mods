"""Check the source still-filled Station row and take-only interaction."""
import argparse
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for key in ('game', 'ctl', 'resources'): p.add_argument('--' + key, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--lan', action='store_true')
p.add_argument('--in-frame', action='store_true')
a = p.parse_args(); run = a.run_dir.resolve(); games = []

def click(game, slot, *flags):
    game.call('click-slot', str(slot), *flags); game.call('wait', '10')
def hotbar(game, item):
    return next(row['slot'] for row in game.call('inventory')['hotbar'] if row.get('item') == item)
def stack(game, slot):
    return next((row.get('stack') for row in game.call('menu')['slots'] if row.get('slot') == slot), None)
def carried(game): return (game.call('menu').get('carried') or {}).get('item')
def row(game, slot): return next(w for w in game.call('ui')['widgets'] if w.get('id') == f'slot-{slot}')

try:
    host = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); games.append(host); host.ready()
    if a.lan:
        port = host.call('lan', 'open', '0', 'commands')['port']
        actor = Game(a, run, 'Guest', server='127.0.0.1:' + str(port), resume=True); games.append(actor); actor.ready()
        commands = ('gamemode creative @a', 'clear @a', 'tp @s -2.5 71 0.5', 'tp Guest 0.5 71 0.5')
        target = 'Guest'
    else:
        actor = host; commands = ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5'); target = '@s'
    for command in (*commands, 'setblock 0 71 1 minecraft:air', 'setblock 0 71 2 minecraft:air',
                    'setblock 0 71 2 tconstruct:tinker_station',
                    f'give {target} tconstruct:tool_binding[minecraft:custom_data={{Material:"tconstruct:wood"}}] 1',
                    f'give {target} minecraft:stone 1', f'give {target} minecraft:iron_helmet 1'):
        host.call('command', command)
    host.call('wait', '15'); actor.call('interact', '0', '71', '2'); actor.call('wait', '20')
    # Repair layout exposes all inputs; change to the two-input dagger layout.
    actor.call('click-gui', '-121', '18'); actor.call('wait', '8')
    click(actor, hotbar(actor, 'tconstruct:tool_binding')); click(actor, 302)
    actor.call('click-gui', '-99', '18'); actor.call('wait', '20')
    assert stack(actor, 302) and row(actor, 302)['bounds'][0] >= 0, 'occupied dormant slot was hidden'
    click(actor, hotbar(actor, 'minecraft:stone')); click(actor, 302)
    assert carried(actor) == 'minecraft:stone' and stack(actor, 302), 'dormant slot accepted an item'
    empty = next(x['slot'] for x in actor.call('inventory')['hotbar'] if not x.get('item'))
    click(actor, empty); click(actor, 302)
    assert carried(actor) == 'tconstruct:tool_binding', 'dormant item could not be taken'
    click(actor, 302)  # inactive empty slot must reject re-insertion
    assert carried(actor) == 'tconstruct:tool_binding' and not stack(actor, 302)
    empty = next(x['slot'] for x in actor.call('inventory')['hotbar'] if not x.get('item'))
    click(actor, empty)
    actor.call('click-gui', '-121', '18'); actor.call('wait', '8')
    click(actor, hotbar(actor, 'tconstruct:tool_binding')); click(actor, 302)
    actor.call('click-gui', '-99', '18'); actor.call('wait', '20')
    click(actor, 302, 'shift')
    assert not stack(actor, 302) and actor.call('inventory')['counts'].get('tconstruct:tool_binding', 0) == 1, 'shift take failed'
    click(actor, hotbar(actor, 'minecraft:iron_helmet')); click(actor, 500)
    actor.call('wait', '20'); actor.call('screenshot', str(run / ('station-dormant-lan.png' if a.lan else 'station-dormant.png')))
    actor.call('close'); host.call('save')
    for game in reversed(games): game.stop()
    print('PASS dormant input take-only, shift and equipment preview', flush=True)
finally:
    for game in reversed(games): game.cleanup()

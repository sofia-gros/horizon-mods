"""Exercise Smeltery bucket slots, direct transfer, and returned containers.

Only the named structure positions in the existing test world are reset.
"""
import argparse
import subprocess
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
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=(run / 'Player').exists())


def hotbar(item):
    return next(slot['slot'] for slot in g.call('inventory')['hotbar'] if slot.get('item') == item)


def select(item):
    g.call('hotbar', str(hotbar(item) + 1)); g.call('wait', '10')


def empty_hand():
    slot = next(slot['slot'] for slot in g.call('inventory')['hotbar'] if not slot.get('item'))
    g.call('hotbar', str(slot + 1)); g.call('wait', '10')


def click(slot):
    g.call('click-slot', str(slot)); g.call('wait', '5')


try:
    g.ready()
    # These positions belong to this scenario; remove old machines first so
    # re-running against the shared test world starts with fresh state.
    for x, y, z in ((0,71,1), (1,71,2), (0,71,3), (0,71,4), (0,70,4)):
        g.call('command', f'setblock {x} {y} {z} minecraft:air')
    g.call('wait', '10')
    for command in (
        'gamemode creative @s', 'clear @s', 'tp @s 2.5 71 1.5',
        'setblock 0 70 2 tconstruct:seared_bricks',
        'setblock 0 71 2 minecraft:air',
        'setblock -1 71 2 tconstruct:seared_bricks',
        'setblock 1 71 2 tconstruct:seared_fuel_tank',
        'setblock 0 71 3 tconstruct:seared_drain[facing=south]',
        'setblock 0 71 1 tconstruct:smeltery_controller[facing=north,lit=false]',
        'setblock 0 71 4 tconstruct:seared_faucet[facing=south]',
        'setblock 0 70 4 tconstruct:seared_table',
        'give @s minecraft:lava_bucket 1', 'give @s tconstruct:molten_copper_bucket 1', 'give @s minecraft:copper_ingot 1',
        'give @s minecraft:amethyst_shard 1', 'give @s tconstruct:ingot_cast 1',
    ):
        g.call('command', command)
    g.call('wait', '20')
    select('minecraft:lava_bucket'); g.call('interact', '1', '71', '2'); g.call('wait', '10')
    g.call('command', 'tp @s 0.5 71 -1.5'); g.call('wait', '10')
    empty_hand()
    try:
        g.call('interact', '0', '71', '1')
    except subprocess.CalledProcessError as error:
        print(f'controller interaction failed: {error.stdout}\n{error.stderr}', flush=True)
        raise
    g.call('wait', '20')
    assert g.call('status')['screen'] == 'inventory', 'formed Smeltery menu did not open'
    assert any(widget.get('id') == 'slot-302' for widget in g.call('ui')['widgets']), 'real input slot is missing'
    g.call('click-slot', str(hotbar('tconstruct:molten_copper_bucket')), 'shift'); g.call('wait', '5')
    assert any(w.get('id') == 'slot-301' and (w.get('stack') or {}).get('item') == 'minecraft:bucket' for w in g.call('ui')['widgets']), 'empty bucket was not returned'
    node = next(row for row in g.call('ui')['widgets'] if row.get('id') == 'slot-301')
    x, y, _, _ = node['bounds']; ox, oy = x - 125, y - 104
    g.call('wait', '220')  # Let setup chat fade before saving documentation images.
    for name, px, py in (('mode', 130, 75), ('fuel', 160, 80), ('fluid', 30, 115)):
        g.call('scroll-at', str(ox + px), str(oy + py), '0'); g.call('wait', '5')
        g.call('screenshot', str(run / ('gui-smeltery-tooltip-' + name + '.png')))
    click(301); click(300)
    assert any(w.get('id') == 'slot-301' and (w.get('stack') or {}).get('item') == 'tconstruct:molten_copper_bucket' for w in g.call('ui')['widgets']), 'filled bucket was not returned'
    click(301)
    # Put the filled bucket away and show the active input's source heat bar.
    empty = next(slot['slot'] for slot in g.call('inventory')['hotbar'] if not slot.get('item'))
    click(empty); click(hotbar('minecraft:copper_ingot')); click(302); g.call('wait', '30')
    g.call('scroll-at', str(ox - 23), str(oy + 20), '0'); g.call('wait', '5')
    g.call('screenshot', str(run / 'gui-smeltery-heating.png'))
    # The filled bucket remains in the inventory across save and reload.
    g.call('close'); g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    assert g.call('inventory')['counts'].get('tconstruct:molten_copper_bucket', 0) == 1, 'filled bucket was lost on close/reload'
    g.call('save'); g.stop()
    print('PASS bucket input, empty return, filling and close/reload', flush=True)

finally:
    g.cleanup()

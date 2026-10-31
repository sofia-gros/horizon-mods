"""Exercise source Smeltery copper+amethyst alloy and casting in the test world.

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
        'give @s minecraft:lava_bucket 1', 'give @s minecraft:copper_ingot 1',
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
    g.call('screenshot', str(run / 'smeltery-input-debug.png'))
    print('input widget:', [w for w in g.call('ui')['widgets'] if w.get('id') in ('slot-300','slot-301','slot-302','slot-303')], flush=True)
    click(hotbar('minecraft:copper_ingot')); click(302)
    print('input after insertion:', g.call('menu'), flush=True)
    print('input widget after insertion:', [w for w in g.call('ui')['widgets'] if w.get('id') == 'slot-302'], flush=True)
    g.call('screenshot', str(run / 'smeltery-input-after.png'))
    # An interrupted structure must retain its input, then resume after repair.
    g.call('wait', '20'); g.call('close')
    g.call('command', 'setblock -1 71 2 minecraft:air'); g.call('wait', '40')
    g.call('command', 'setblock -1 71 2 tconstruct:seared_bricks'); g.call('wait', '20')
    g.call('interact', '0', '71', '1'); g.call('wait', '10')
    repaired = g.call('menu')
    print('input after repair:', repaired, flush=True)
    print('input widget after repair:', [w for w in g.call('ui')['widgets'] if w.get('id') == 'slot-302'], flush=True)
    assert any(w.get('id') == 'slot-302' and (w.get('stack') or {}).get('item') == 'minecraft:copper_ingot' for w in g.call('ui')['widgets']), 'input lost on structure repair'
    g.call('wait', '300', timeout=40)
    assert not any(row.get('slot') == 302 and row.get('stack') for row in g.call('menu')['slots']), 'copper did not melt'
    click(hotbar('minecraft:amethyst_shard')); click(302)
    g.call('wait', '360', timeout=45)
    assert not any(row.get('slot') == 302 and row.get('stack') for row in g.call('menu')['slots']), 'amethyst did not melt'
    g.call('wait', '20'); g.call('screenshot', str(run / 'smeltery-alloy.png'))
    g.call('close'); g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    g.call('command', 'tp @s 2.5 71 4.5'); g.call('wait', '10')
    select('tconstruct:ingot_cast'); g.call('interact', '0', '70', '4'); g.call('wait', '5')
    empty_hand(); g.call('interact', '0', '71', '4'); g.call('wait', '120', timeout=25)
    g.call('screenshot', str(run / 'smeltery-casting.png'))
    g.call('interact', '0', '70', '4'); g.call('wait', '5')
    assert g.call('inventory')['counts'].get('tconstruct:amethyst_bronze_ingot', 0) == 1, 'alloy casting result missing'
    g.call('save'); g.stop()
    print('PASS lava fuel, copper and amethyst alloy, save/reload and cast ingot', flush=True)
finally:
    g.cleanup()

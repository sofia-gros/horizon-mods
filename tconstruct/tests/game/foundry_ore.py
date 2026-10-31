"""Exercise one source Foundry ore, byproduct selection, and casting."""
import argparse
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


def input_item():
    return next((w.get('stack') for w in g.call('ui')['widgets'] if w.get('id') == 'slot-302'), None)


try:
    g.ready()
    # Only this scenario's machine locations are reset in the shared test world.
    for x, y, z in ((0,71,1), (1,71,2), (0,71,3), (0,71,4), (0,70,4)):
        g.call('command', f'setblock {x} {y} {z} minecraft:air')
    g.call('wait', '10')
    for command in ('gamemode creative @s', 'clear @s', 'tp @s 2.5 71 1.5'):
        g.call('command', command)
    for x in range(-1, 2):
        for z in range(1, 4):
            g.call('command', f'setblock {x} 70 {z} tconstruct:scorched_bricks')
            if x != 0 or z != 2:
                g.call('command', f'setblock {x} 71 {z} tconstruct:scorched_bricks')
    for command in (
        'setblock 0 71 2 minecraft:air',
        'setblock 1 71 2 tconstruct:scorched_fuel_tank',
        'setblock 0 71 3 tconstruct:scorched_drain[facing=south]',
        'setblock 0 71 1 tconstruct:foundry_controller[facing=north,lit=false]',
        'setblock 0 71 4 tconstruct:seared_faucet[facing=south]',
        'setblock 0 70 4 tconstruct:seared_table',
        'give @s minecraft:lava_bucket 1', 'give @s minecraft:copper_ore 1',
        'give @s tconstruct:ingot_cast 1',
    ):
        g.call('command', command)
    g.call('wait', '20')
    select('minecraft:lava_bucket'); g.call('interact', '1', '71', '2'); g.call('wait', '10')
    g.call('command', 'tp @s 0.5 71 -1.5'); g.call('wait', '10')
    empty_hand(); g.call('interact', '0', '71', '1'); g.call('wait', '20')
    assert g.call('status')['screen'] == 'inventory', 'formed Foundry did not open'
    click(hotbar('minecraft:copper_ore')); click(302)
    assert (input_item() or {}).get('item') == 'minecraft:copper_ore', 'ore input was not accepted'
    g.call('wait', '1100', timeout=85)
    g.call('screenshot', str(run / 'foundry-ore-check.png'))
    g.call('save')
    assert input_item() is None, 'copper ore did not melt'
    g.call('wait', '20'); g.call('screenshot', str(run / 'foundry-ore.png'))
    # Source minimum-height fluid layers make the small gold byproduct selectable.
    g.call('click-gui', '30', '115.5'); g.call('wait', '10')
    g.call('screenshot', str(run / 'foundry-byproduct-selected.png'))
    # Return copper to the bottom before casting it.
    g.call('click-gui', '30', '115.5'); g.call('wait', '10')
    g.call('close'); g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    g.call('command', 'tp @s 2.5 71 4.5'); g.call('wait', '10')
    select('tconstruct:ingot_cast'); g.call('interact', '0', '70', '4'); g.call('wait', '5')
    empty_hand(); g.call('interact', '0', '71', '4'); g.call('wait', '120', timeout=25)
    g.call('interact', '0', '70', '4'); g.call('wait', '5')
    assert g.call('inventory')['counts'].get('minecraft:copper_ingot', 0) == 1, 'copper cast missing'
    g.call('save'); g.stop()
    print('PASS Foundry ore, byproduct selection, save/reload and copper cast', flush=True)
finally:
    g.cleanup()

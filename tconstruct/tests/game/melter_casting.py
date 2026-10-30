"""Exercise Melter -> faucet -> table/basin in the reusable test world.

This uses the test world's existing workstation coordinates and only resets the
five blocks used by this scenario. It does not remove a world or any cache.
"""
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


def click(slot):
    g.call('click-slot', str(slot))
    g.call('wait', '5')


def hotbar(item):
    return next(slot['slot'] for slot in g.call('inventory')['hotbar'] if slot.get('item') == item)


def select_empty():
    slot = next(slot['slot'] for slot in g.call('inventory')['hotbar'] if not slot.get('item'))
    g.call('hotbar', str(slot + 1)); g.call('wait', '10')


try:
    g.ready()
    for command in (
        'gamemode creative @s', 'clear @s', 'tp @s 2.5 71 1.5',
        'setblock 0 72 2 minecraft:air', 'setblock 0 71 2 minecraft:air',
        'setblock 0 72 1 minecraft:air', 'setblock 0 71 1 minecraft:air',
        'setblock 0 71 2 tconstruct:seared_heater',
    ): g.call('command', command)
    select_empty(); g.call('interact', '0', '71', '2'); g.call('wait', '5')
    assert g.call('status')['screen'] == 'inventory', 'heater menu did not open'
    g.call('close')
    for command in (
        'setblock 0 72 2 tconstruct:seared_melter',
        'setblock 0 72 1 tconstruct:seared_faucet[facing=north]',
        'setblock 0 71 1 tconstruct:seared_table',
        'give @s minecraft:coal 2', 'give @s minecraft:copper_ingot 10',
        'give @s tconstruct:ingot_cast 1',
    ):
        g.call('command', command)
    # /setblock does not create custom entities, so exercise the normal
    # interaction that initializes the heater before opening the melter.
    g.call('interact', '0', '72', '2'); g.call('wait', '10')
    assert any(w.get('id') == 'slot-327' for w in g.call('ui')['widgets']), 'heater slot is missing'
    click(hotbar('minecraft:coal')); click(327)
    click(hotbar('minecraft:copper_ingot')); click(300)
    for _ in range(7): g.call('wait', '400', timeout=45)
    g.call('screenshot', str(run / 'melter-filled.png'))
    assert not any(slot.get('slot') == 300 and slot.get('stack') for slot in g.call('menu')['slots'])
    g.call('close'); g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    g.call('hotbar', str(hotbar('tconstruct:ingot_cast') + 1)); g.call('wait', '10')
    g.call('interact', '0', '71', '1'); g.call('wait', '5')
    select_empty(); g.call('interact', '0', '72', '1'); g.call('wait', '120', timeout=45)
    g.call('screenshot', str(run / 'casting-table.png'))
    g.call('interact', '0', '71', '1'); g.call('wait', '5')
    assert g.call('inventory')['counts'].get('minecraft:copper_ingot', 0) == 1
    # Replace the test table only after taking its result and cast.
    g.call('interact', '0', '71', '1'); g.call('wait', '5')
    g.call('command', 'setblock 0 71 1 tconstruct:seared_basin')
    select_empty(); g.call('interact', '0', '72', '1'); g.call('wait', '500', timeout=45)
    g.call('screenshot', str(run / 'casting-basin.png'))
    g.call('interact', '0', '71', '1'); g.call('wait', '5')
    assert g.call('inventory')['counts'].get('minecraft:copper_block', 0) == 1
    # Breaking the machine must return its live contents. Replacing it at the
    # same position must not revive the old tank or inputs.
    g.call('command', 'clear @s')
    g.call('command', 'give @s minecraft:raw_copper 3')
    g.call('interact', '0', '72', '2'); g.call('wait', '5')
    click(hotbar('minecraft:raw_copper')); click(300); g.call('close')
    g.call('command', 'give @s minecraft:iron_pickaxe 1')
    g.call('hotbar', str(hotbar('minecraft:iron_pickaxe') + 1)); g.call('wait', '10')
    g.call('command', 'gamemode survival @s')
    g.call('break', '0', '72', '2', '--timeout', '40', timeout=45)
    g.call('command', 'tp @s 0.5 72 2.5'); g.call('wait', '80')
    assert g.call('inventory')['counts'].get('minecraft:raw_copper', 0) == 3
    g.call('command', 'gamemode creative @s'); g.call('command', 'tp @s 2.5 71 1.5')
    g.call('command', 'setblock 0 72 2 tconstruct:seared_melter')
    select_empty(); g.call('interact', '0', '72', '2'); g.call('wait', '10')
    assert not any(row.get('slot') in (300, 301, 302) and row.get('stack') for row in g.call('menu')['slots'])
    g.call('close'); g.call('save'); g.stop()
    print('PASS melter fuel, save/reload, faucet, ingot cast and basin block', flush=True)
finally:
    g.cleanup()

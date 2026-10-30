"""Check that a full Melter does not consume additional heater fuel.

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
        'setblock 0 72 1 tconstruct:seared_faucet[facing=north]', 'setblock 0 71 1 tconstruct:seared_table',
        'give @s minecraft:coal 2', 'give @s minecraft:copper_ingot 25', 'give @s tconstruct:ingot_cast 1',
    ):
        g.call('command', command)
    g.call('interact', '0', '72', '2'); g.call('wait', '10')
    assert any(w.get('id') == 'slot-327' for w in g.call('ui')['widgets']), 'heater slot is missing'
    click(hotbar('minecraft:coal')); click(327)
    click(hotbar('minecraft:copper_ingot'))
    for slot in (300, 301):
        for _ in range(8): g.call('click-slot', str(slot), 'right')
    click(302)
    for _ in range(9): g.call('wait', '400', timeout=45)
    values = {row['slot']: row['stack'] for row in g.call('menu')['slots'] if row.get('stack')}
    assert 327 not in values and sum(values.get(slot, {}).get('count', 0) for slot in (300, 301, 302)) == 1, values
    g.call('command', 'give @s minecraft:coal 1')
    click(hotbar('minecraft:coal')); click(327)
    g.call('wait', '100', timeout=20)
    values = {row['slot']: row['stack'] for row in g.call('menu')['slots'] if row.get('stack')}
    assert values.get(327, {}).get('item') == 'minecraft:coal', values
    g.call('close'); g.call('hotbar', str(hotbar('tconstruct:ingot_cast') + 1)); g.call('wait', '10')
    g.call('interact', '0', '71', '1'); select_empty(); g.call('interact', '0', '72', '1')
    g.call('wait', '120', timeout=25)
    g.call('interact', '0', '72', '2'); g.call('wait', '10')
    values = {row['slot']: row['stack'] for row in g.call('menu')['slots'] if row.get('stack')}
    assert values.get(327, {}).get('item') == 'minecraft:coal', values
    assert not any(slot in values for slot in (300, 301, 302)), values
    g.call('screenshot', str(run / 'melter-full-fuel.png'))
    g.call('close'); g.stop()
    print('PASS full tank retains fuel and resumes after draining', flush=True)
finally:
    g.cleanup()

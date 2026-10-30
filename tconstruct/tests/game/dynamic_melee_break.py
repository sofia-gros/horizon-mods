"""Craft a material-derived mattock, break it by melee, repair and reload."""
import argparse
import time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
a = p.parse_args()
run = a.run_dir.resolve()
run.mkdir(parents=True, exist_ok=True)
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=(run / 'Player').exists())

def click(slot):
    g.call('click-slot', str(slot)); g.call('wait', '5')

def hotbar(item):
    return next(s['slot'] for s in g.call('inventory')['hotbar'] if s.get('item') == item)

try:
    g.ready()
    commands = ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5',
                'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 tconstruct:tinker_station',
                'give @s tconstruct:small_axe_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
                'give @s tconstruct:tool_handle[minecraft:custom_data={Material:"tconstruct:wood"}] 1',
                'give @s tconstruct:adze_head[minecraft:custom_data={Material:"tconstruct:flint"}] 1',
                'give @s minecraft:flint 1')
    for command in commands: g.call('command', command)
    g.call('interact', '0', '71', '2'); g.call('wait', '10')
    for i, item in enumerate(('tconstruct:small_axe_head', 'tconstruct:tool_handle', 'tconstruct:adze_head')):
        click(hotbar(item)); click(300 + i)
    click(324)
    assert (g.call('menu')['carried'] or {}).get('item') == 'tconstruct:mattock'
    click(0); g.call('close'); g.call('hotbar', '1')
    held = g.call('inventory')['held']
    assert 'tic_max_damage:106' in held.get('components', ''), held
    # set_damage operates on the registered maximum (131): 0.194 leaves 105 damage.
    g.call('command', 'item modify entity @s weapon.mainhand {type:"minecraft:set_damage",function:"minecraft:set_damage",damage:0.194}')
    for _ in range(40):
        held = g.call('inventory')['held']
        if held.get('damage') == 105: break
        g.call('wait', '3')
    assert held.get('damage') == 105, held
    g.call('command', 'gamemode survival @s'); g.call('command', 'summon minecraft:zombie 1.5 71 0.5')
    zombie = None
    for _ in range(30):
        entities = g.call('entities', '8', 'minecraft:zombie')['entities']
        if entities: zombie = entities[0]['id']; break
        time.sleep(.2)
    assert zombie is not None, 'zombie not available'
    for _ in range(5):
        g.call('attack', str(zombie)); g.call('wait', '30')
        if g.call('inventory')['held'].get('damage') == 106: break
    held = g.call('inventory')['held']
    assert held.get('item') == 'tconstruct:mattock' and held.get('damage') == 106 and 'tic_broken:1b' in held.get('components', ''), held
    g.call('command', 'gamemode creative @s'); g.call('interact', '0', '71', '2'); g.call('wait', '10')
    click(0); click(300); click(hotbar('minecraft:flint')); click(301); click(325)
    repaired = g.call('menu')['carried'] or {}
    assert repaired.get('item') == 'tconstruct:mattock' and repaired.get('damage', 106) < 106, repaired
    click(0); g.call('screenshot', str(run / 'mattock-melee-repaired.png')); g.call('close')
    g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    held = g.call('inventory')['held']
    assert held.get('item') == 'tconstruct:mattock' and held.get('damage', 106) < 106 and 'tic_broken:0b' in held.get('components', ''), held
    g.stop(); print('PASS crafted mattock melee break, repair and reload', flush=True)
finally:
    g.cleanup()

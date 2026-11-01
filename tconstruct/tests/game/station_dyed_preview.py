"""Compare undyed and source-shaped synthetic dyed Plate preview stacks.

This is a display fixture, not a currently obtainable dye recipe.
"""
import argparse
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for key in ('game', 'ctl', 'resources'):
    p.add_argument('--' + key, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
a = p.parse_args(); run = a.run_dir.resolve()
game = Game(a, run, 'Player', resume=True)
try:
    game.ready()
    for command in ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5',
                    'setblock 0 71 1 minecraft:air', 'setblock 0 71 2 minecraft:air',
                    'setblock 0 71 2 tconstruct:tinker_station',
                    'give @s tconstruct:plate_chestplate[minecraft:max_damage=382,minecraft:custom_data={tic_max_damage:382,tic_materials:["tconstruct:steel","tconstruct:steel"]}] 1',
                    'give @s tconstruct:plate_chestplate[minecraft:max_damage=382,minecraft:custom_data={tic_max_damage:382,tic_materials:["tconstruct:steel","tconstruct:steel"],tic_modifiers:[{name:"tconstruct:dyed",level:1}],tic_persistent:{"tconstruct:dyed":16711680}}] 1'):
        game.call('command', command)
    game.call('wait', '15'); game.call('interact', '0', '71', '2'); game.call('wait', '15')
    game.call('click-gui', '-121', '18'); game.call('wait', '220')
    slots = [x['slot'] for x in game.call('inventory')['hotbar'] if x.get('item') == 'tconstruct:plate_chestplate']
    assert len(slots) == 2, slots
    for slot, name in zip(slots, ('undyed', 'dyed')):
        game.call('click-slot', str(slot)); game.call('click-slot', '300'); game.call('wait', '20')
        item = next(x for x in game.call('menu')['slots'] if x['slot'] == 300)
        assert (item.get('stack') or {}).get('item') == 'tconstruct:plate_chestplate', item
        game.call('screenshot', str(run / ('station-maille-' + name + '.png')))
        game.call('click-slot', '300'); game.call('click-slot', str(slot)); game.call('wait', '10')
    game.call('close'); game.call('save'); game.stop()
    print('PASS synthetic dyed and undyed preview inputs; inspect screenshots', flush=True)
finally:
    game.cleanup()

"""Check representative registered equipment in the existing isolated world."""
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
try:
    g.ready()
    g.call('command', 'gamemode creative @s')
    g.call('command', 'clear @s')
    equipment = ('tconstruct:broad_axe', 'tconstruct:dagger', 'tconstruct:plate_helmet', 'tconstruct:sky_staff')
    for item in equipment:
        g.call('command', f'give @s {item} 1')
    for _ in range(40):
        inventory = g.call('inventory')
        if all(inventory['counts'].get(item) == 1 for item in equipment):
            break
        g.call('wait', '5')
    for item in equipment:
        assert inventory['counts'].get(item) == 1, (item, inventory)
    g.call('command', 'gamemode survival @s')
    g.call('wait', '10')
    g.call('open-inventory')
    g.call('wait', '10')
    g.call('screenshot', str(run / 'equipment-inventory.png'))
    slot = next(s['slot'] for s in inventory['hotbar'] if s.get('item') == 'tconstruct:plate_helmet')
    g.call('click-slot', str(slot))
    g.call('click-slot', '500')
    g.call('wait', '10')
    inventory = g.call('inventory')
    assert inventory['armor'][0].get('item') == 'tconstruct:plate_helmet', inventory['armor']
    g.call('screenshot', str(run / 'equipment-equipped.png'))
    g.call('close')
    g.call('save')
    g.stop()
    print('PASS representative registered equipment and helmet slot', flush=True)
finally:
    g.cleanup()

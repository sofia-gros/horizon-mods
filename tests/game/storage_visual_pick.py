"""Exercise the registered material block's display and local Pick Block."""
import argparse, json, tempfile, time
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='storage-visual-pick-', dir=Path('target').resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
try:
    g.ready()
    pack = g.root / 'saves/Probe/datapacks/storage-test'
    path = pack / 'data/tconstruct/tinkering/materials/definition/bronze.json'
    path.parent.mkdir(parents=True)
    path.write_text(json.dumps({'craftable': True}))
    (pack / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"storage test"}}')
    g.call('command', 'reload')
    for cmd in ('fill -2 70 -1 4 70 5 minecraft:stone', 'tp @s 0.5 71 0.5', 'clear @s',
                'give @s tconstruct:fake_storage_block[minecraft:custom_data={Material:"tconstruct:bronze"}] 1'):
        g.call('command', cmd)
    g.call('wait', '40')
    g.call('place', '0', '71', '2', '--item', 'tconstruct:fake_storage_block')
    g.call('command', 'clear @s')
    g.call('command', 'give @s tconstruct:fake_storage_block[minecraft:custom_data={Material:"tconstruct:unknown"}] 1')
    g.call('wait', '20')
    g.call('place', '1', '71', '2', '--item', 'tconstruct:fake_storage_block')
    g.call('wait', '80')
    g.call('look-at', 'block', '0', '71', '2')
    g.call('screenshot', str(run / 'bronze.png'))
    g.call('command', 'clear @s')
    g.call('mouse', 'middle', 'click')
    g.call('wait', '20')
    slots = g.call('inventory')['hotbar']
    assert any(s.get('item') == 'tconstruct:fake_storage_block' and 'tconstruct:bronze' in s.get('components', '') for s in slots), slots
    print('PASS Pick Block preserved bronze', flush=True)
    g.call('save')
    g.stop()
finally:
    g.cleanup()

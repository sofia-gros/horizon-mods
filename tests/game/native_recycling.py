"""Exercise a 26.3-native damageable input through shared recycling."""
import argparse, json, tempfile, time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='native-recycling-', dir=Path('target').resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
def click(slot):
    g.call('click-slot', str(slot)); g.call('wait', '3')
try:
    g.ready()
    pack = g.root / 'saves/Probe/datapacks/native-recycling'
    recipe_path = pack / 'data/tconstruct/recipes/tables/recycling/stone_axe.json'
    recipe_path.parent.mkdir(parents=True)
    (pack / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"native 26.3 recycling"}}')
    source = a.resources / 'tconstruct-original-pack/data/tconstruct/recipes/tables/recycling/stone_axe.json'
    recipe = json.loads(source.read_text()); recipe['tool'] = {'item': 'minecraft:mace'}
    recipe_path.write_text(json.dumps(recipe))
    g.call('command', 'reload'); g.call('wait', '60')
    for cmd in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder',
                'give @s tconstruct:venom_bottle 1', 'give @s minecraft:mace[minecraft:damage=250] 1'):
        g.call('command', cmd)
    g.call('interact', '0', '71', '2'); click(0); click(300); click(1); click(301)
    for _ in range(120):
        node = next((n for n in g.call('ui')['widgets'] if n.get('label') == 'Block' and n.get('enabled')), None)
        if node: break
        time.sleep(.2)
    assert node, g.call('ui')
    g.call('click', node['id']); g.call('wait', '20'); click(322)
    result = g.call('menu').get('carried') or {}
    assert result.get('item') == 'minecraft:cobblestone' and result.get('count') == 1, result
    slots = {r['slot']: r['stack'] for r in g.call('menu')['slots'] if r.get('stack')}
    assert slots.get(300, {}).get('item') == 'minecraft:glass_bottle' and 301 not in slots, slots
    click(1); g.call('close'); g.call('save'); g.stop(); g.cleanup()
    g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
    assert g.call('inventory')['counts'].get('minecraft:cobblestone') == 1
    print('PASS 26.3 mace durability, input consumption and save/reload', flush=True)
    g.stop()
finally:
    g.cleanup()

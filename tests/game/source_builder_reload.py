"""Check a table's source recipe lifetime across a datapack reload."""
import argparse, json, tempfile, time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
p.add_argument('--material-update', action='store_true')
p.add_argument('--tag-update', action='store_true')
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='source-builder-reload-', dir=Path('target').resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
def click(slot):
    g.call('click-slot', str(slot)); g.call('wait', '3')
def node():
    return next((n for n in g.call('ui')['widgets'] if n.get('label') == ('Tool Handle' if a.tag_update else 'Pick Head') and n.get('enabled')), None)
try:
    g.ready()
    material_item = 'minecraft:oak_log' if a.tag_update else 'minecraft:flint'
    for command in ('clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 tconstruct:part_builder',
                    'give @s tconstruct:pattern 2', f'give @s {material_item} 3'):
        g.call('command', command)
    g.call('interact', '0', '71', '2')
    click(0); click(300); click(1); click(301)
    print('input slots', g.call('menu').get('slots'), flush=True)
    for _ in range(120):
        if node(): break
        time.sleep(.2)
    assert node(), g.call('ui')
    g.call('click', node()['id']); g.call('wait', '20')
    pack = g.root / 'saves/Probe/datapacks/source-lifetime'
    (pack / 'pack.mcmeta').parent.mkdir(parents=True)
    (pack / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"source lifetime"}}')
    if a.tag_update:
        path = pack / 'data/minecraft/tags/items/oak_logs.json'
        recipe = {'replace': True, 'values': ['minecraft:birch_log']}
    elif a.material_update:
        path = pack / 'data/tconstruct/recipes/tools/materials/flint/flint.json'
        recipe = json.loads((a.resources / 'tconstruct-original-pack/data/tconstruct/recipes/tools/materials/flint/flint.json').read_text())
        assert recipe['value'] == 1, recipe
        recipe['value'] = 2
    else:
        path = pack / 'data/tconstruct/recipes/tools/parts/builder/pick_head.json'
        recipe = json.loads((Path(__file__).resolve().parents[2] / 'content/data/tconstruct/recipes/tools/parts/builder/pick_head.json').read_text())
        assert recipe['cost'] == 2, recipe
        recipe['cost'] = 3
    path.parent.mkdir(parents=True)
    path.write_text(json.dumps(recipe))
    g.call('command', 'reload'); g.call('wait', '60')
    if a.material_update or a.tag_update:
        click(300); click(2); click(2); click(300); g.call('wait', '20')
    click(322)
    assert (g.call('menu').get('carried') or {}).get('item') == ('tconstruct:tool_handle' if a.tag_update else 'tconstruct:pick_head')
    slots = {s['slot']: s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
    assert slots.get(301, {}).get('count') == (2 if a.tag_update else 1), slots
    g.call('screenshot', str(run / 'cached-result.png'))
    if a.material_update or a.tag_update:
        click(3)  # store the first result
        click(301); click(4); click(4); click(301)  # invalidate the material cache
        g.call('wait', '20'); click(322)
        if a.tag_update:
            assert not (g.call('menu').get('carried') or {}), 'excluded material must no longer craft after reinsertion'
            print('PASS cached material across tag change and invalidation', flush=True)
            g.stop()
            raise SystemExit(0)
        assert (g.call('menu').get('carried') or {}).get('item') == 'tconstruct:pick_head'
        slots = {s['slot']: s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
        assert 301 not in slots, slots  # new value 2 supplies the full cost
        click(3)
        g.call('close'); g.call('save'); g.stop(); g.cleanup()
        g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); g.ready()
        assert sum(s.get('count', 0) for s in g.call('inventory')['hotbar'] if s.get('item') == 'tconstruct:pick_head') == 2
    print('PASS cached material value, invalidation and save/reload' if a.material_update else 'PASS cached original cost 2 after reload changed cost to 3', flush=True)
    g.stop()
finally:
    g.cleanup()

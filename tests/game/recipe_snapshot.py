"""Exercise world-scoped datapack priority and reload using the test-only probe."""
import argparse
import tempfile
import time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
p.add_argument('--source-pack-test', action='store_true')
p.add_argument('--vanilla-pack-test', action='store_true')
p.add_argument('--forge-pack-test', action='store_true')
p.add_argument('--full-recipe-test', action='store_true')
p.add_argument('--output-parent', type=Path, default=Path('target'))
a = p.parse_args()
if a.source_pack_test:
    layer = a.resources / 'tconstruct-original-pack/data/test'
    for kind in ('recipes', 'tags/items'):
        path = layer / kind / 'probe.json'
        if path.exists():
            if path.read_text() != 'source-layer': raise RuntimeError(f'refusing to overwrite {path}')
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('source-layer')
run = Path(tempfile.mkdtemp(prefix='recipe-snapshot-', dir=a.output_parent.resolve()))
print('run', run, flush=True)

def pack(g, name, marker):
    base = g.root / 'saves/Probe/datapacks' / name
    (base / 'data/test/recipes').mkdir(parents=True, exist_ok=True)
    (base / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"test"}}')
    (base / 'data/test/recipes/probe.json').write_text(marker)
    (base / 'data/test/tags/items').mkdir(parents=True, exist_ok=True)
    (base / 'data/test/tags/items/probe.json').write_text(marker)

def probe(g, expected, tag_expected=None, planks_expected=None, forge_expected=None, vanilla_enabled=True):
    # The fixture is the inner RETURN modifier, so it observes the previous
    # snapshot. The first reload refreshes it; the second observes that state.
    g.call('command', 'reload')
    before = (g.root / 'game.log').read_text().count('TCONSTRUCT_RECIPE_PROBE ')
    g.call('command', 'reload')
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        lines = (g.root / 'game.log').read_text().splitlines()
        markers = [line.split('TCONSTRUCT_RECIPE_PROBE ', 1)[1] for line in lines if 'TCONSTRUCT_RECIPE_PROBE ' in line]
        if len(markers) > before:
            assert markers[-1] == expected, (markers, expected)
            tags = [line.split('TCONSTRUCT_TAG_PROBE ', 1)[1] for line in lines if 'TCONSTRUCT_TAG_PROBE ' in line]
            assert tags and tags[-1] == (tag_expected or expected), (tags, tag_expected or expected)
            if planks_expected is not None:
                planks = [line.split('TCONSTRUCT_PLANKS_PROBE ', 1)[1] for line in lines if 'TCONSTRUCT_PLANKS_PROBE ' in line]
                assert planks and int(planks[-1]) == planks_expected, (planks, planks_expected)
            if forge_expected is not None:
                forge = [line.split('TCONSTRUCT_FORGE_IRON_PROBE ', 1)[1] for line in lines if 'TCONSTRUCT_FORGE_IRON_PROBE ' in line]
                assert forge and int(forge[-1]) == forge_expected, (forge, forge_expected)
            if a.full_recipe_test:
                def last(marker):
                    values = [line.split(marker, 1)[1] for line in lines if marker in line]
                    assert values, marker
                    return values[-1]
                assert int(last('TCONSTRUCT_COUNT_PROBE ')) == 3067
                assert last('TCONSTRUCT_BOAT_PROBE ') == 'eab22b18cc6dd8af83a80ea388f598b4f0b57c8ba22a1ce0985c44003e71619d'
                stick = '0105045638880cbad58c912fbcac60e01a7d76578c64d0515ce5855316b76f4e' if vanilla_enabled else 'missing'
                assert last('TCONSTRUCT_STICK_PROBE ') == stick
            return
        time.sleep(.1)
    raise TimeoutError('recipe probe log')

games = []
try:
    first = Game(a, run, 'First', in_frame=a.in_frame); games.append(first); first.ready()
    probe(first, 'source-layer' if a.source_pack_test else 'missing', planks_expected=2 if a.vanilla_pack_test else None, forge_expected=1 if a.forge_pack_test else None)
    if a.source_pack_test:
        first.call('command', 'datapack disable vanilla')
        probe(first, 'source-layer', planks_expected=1 if a.vanilla_pack_test else None, forge_expected=1 if a.forge_pack_test else None, vanilla_enabled=False)
    pack(first, 'a', 'world-one-a')
    probe(first, 'world-one-a', 'source-layer|world-one-a' if a.source_pack_test else None, vanilla_enabled=not a.source_pack_test)
    pack(first, 'b', 'world-one-b')
    probe(first, 'world-one-b', 'source-layer|world-one-a|world-one-b' if a.source_pack_test else 'world-one-a|world-one-b', vanilla_enabled=not a.source_pack_test)
    first.call('command', 'datapack disable "file/b"')
    probe(first, 'world-one-a', 'source-layer|world-one-a' if a.source_pack_test else None, vanilla_enabled=not a.source_pack_test)
    second = Game(a, run, 'Second', in_frame=a.in_frame); games.append(second); second.ready()
    pack(second, 'a', 'world-two-a')
    probe(second, 'world-two-a', 'source-layer|world-two-a' if a.source_pack_test else None)
    probe(first, 'world-one-a', 'source-layer|world-one-a' if a.source_pack_test else None, vanilla_enabled=not a.source_pack_test)
    for game in games: game.stop()
    print('PASS world separation, reload, priority, disable', flush=True)
finally:
    for game in games: game.cleanup()

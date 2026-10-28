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
p.add_argument('--output-parent', type=Path, default=Path('target'))
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='recipe-snapshot-', dir=a.output_parent.resolve()))
print('run', run, flush=True)

def pack(g, name, marker):
    base = g.root / 'saves/Probe/datapacks' / name
    (base / 'data/test/recipes').mkdir(parents=True, exist_ok=True)
    (base / 'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"test"}}')
    (base / 'data/test/recipes/probe.json').write_text(marker)
    (base / 'data/test/tags/items').mkdir(parents=True, exist_ok=True)
    (base / 'data/test/tags/items/probe.json').write_text(marker)

def probe(g, expected, tag_expected=None):
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
            return
        time.sleep(.1)
    raise TimeoutError('recipe probe log')

games = []
try:
    first = Game(a, run, 'First', in_frame=a.in_frame); games.append(first); first.ready()
    probe(first, 'missing')
    pack(first, 'a', 'world-one-a')
    probe(first, 'world-one-a')
    pack(first, 'b', 'world-one-b')
    probe(first, 'world-one-b', 'world-one-a|world-one-b')
    first.call('command', 'datapack disable "file/b"')
    probe(first, 'world-one-a')
    second = Game(a, run, 'Second', in_frame=a.in_frame); games.append(second); second.ready()
    pack(second, 'a', 'world-two-a')
    probe(second, 'world-two-a')
    probe(first, 'world-one-a')
    for game in games: game.stop()
    print('PASS world separation, reload, priority, disable', flush=True)
finally:
    for game in games: game.cleanup()

"""Exercise world-scoped datapack priority and reload using the test-only probe."""
import argparse
import json
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
p.add_argument('--lookup-test', action='store_true')
p.add_argument('--material-pack-test', action='store_true')
p.add_argument('--choices-test', action='store_true')
p.add_argument('--world-tags-test', action='store_true')
p.add_argument('--world-config-test', action='store_true')
p.add_argument('--pattern-atlas-test', action='store_true')
p.add_argument('--output-parent', type=Path, default=Path('target'))
a = p.parse_args()
if a.source_pack_test:
    layer = a.resources / 'tconstruct-original-pack/data/test'
    for kind in ('recipes', 'tags/items', 'tinkering/materials/definition', 'tinkering/tags/materials'):
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
    for kind in ('tinkering/materials/definition', 'tinkering/tags/materials'):
        (base / 'data/test' / kind).mkdir(parents=True, exist_ok=True)
        (base / 'data/test' / kind / 'probe.json').write_text(marker)
    if a.choices_test:
        original = Path(__file__).resolve().parents[2] / 'content/data/tconstruct/recipes/tools/parts/builder/pick_head.json'
        recipe = json.loads(original.read_text())
        recipe['cost'] = 3 if name == 'a' else 1
        path = base / 'data/tconstruct/recipes/tools/parts/builder/pick_head.json'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(recipe))
    if a.world_tags_test:
        path = base / 'data/minecraft/tags/items/oak_logs.json'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({'replace': True, 'values': ['minecraft:oak_log' if name == 'a' else 'minecraft:stripped_oak_log']}))

def probe(g, expected, tag_expected=None, planks_expected=None, forge_expected=None, vanilla_enabled=True, choice_layer="base", config_expected="default"):
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
            if a.world_tags_test:
                values = [line.split('TCONSTRUCT_RESOLVED_OAK ', 1)[1] for line in lines if 'TCONSTRUCT_RESOLVED_OAK ' in line]
                if choice_layer == 'a': expected_oak = 'true false true'
                elif choice_layer == 'b': expected_oak = 'false true true'
                elif vanilla_enabled: expected_oak = 'true true true'
                else: expected_oak = 'false false true'
                assert values and values[-1] == expected_oak, (values, expected_oak)
            if a.world_config_test:
                configs = [line.split('TCONSTRUCT_CONFIG_PROBE ', 1)[1] for line in lines if 'TCONSTRUCT_CONFIG_PROBE ' in line]
                assert configs and (configs[-1] == 'false true' if config_expected == 'default' else configs[-1].startswith('unsupported ')), configs
                flint = [line.split('TCONSTRUCT_REAL_LOOKUP_FLINT ', 1)[1] for line in lines if 'TCONSTRUCT_REAL_LOOKUP_FLINT ' in line]
                oak = [line.split('TCONSTRUCT_REAL_LOOKUP_OAK_LOG ', 1)[1] for line in lines if 'TCONSTRUCT_REAL_LOOKUP_OAK_LOG ' in line]
                if config_expected == 'default':
                    assert flint and flint[-1] == 'tconstruct:tools/materials/flint/flint tconstruct:flint 1 1', flint
                    expect_oak = choice_layer == 'a' or choice_layer == 'base' and vanilla_enabled
                    expected_oak = 'tconstruct:tools/materials/wood/logs/oak tconstruct:wood 4 1' if expect_oak else 'none'
                    assert oak and oak[-1] == expected_oak, (oak, expected_oak)
                    flint_plan = [line.split('TCONSTRUCT_REAL_PLAN_FLINT ', 1)[1] for line in lines if 'TCONSTRUCT_REAL_PLAN_FLINT ' in line]
                    oak_plan = [line.split('TCONSTRUCT_REAL_PLAN_OAK_LOG ', 1)[1] for line in lines if 'TCONSTRUCT_REAL_PLAN_OAK_LOG ' in line]
                    cost = {'base': 2, 'a': 3, 'b': 1}[choice_layer] if a.choices_test else 2
                    assert flint_plan and flint_plan[-1] == f'{cost} 1 0', flint_plan
                    expected_plan = f'1 1 {4 - cost}' if expect_oak else 'none'
                    assert oak_plan and oak_plan[-1] == expected_plan, (oak_plan, expected_plan)
                else:
                    assert flint and flint[-1] == 'unavailable', flint
            if a.material_pack_test:
                def material_last(marker):
                    values = [line.split(marker, 1)[1] for line in lines if marker in line]
                    assert values, marker
                    return values[-1]
                assert material_last('TCONSTRUCT_MATERIAL_PROBE ') == expected
                assert material_last('TCONSTRUCT_MATERIAL_TAG_PROBE ') == (tag_expected or expected)
                assert int(material_last('TCONSTRUCT_MATERIAL_COUNT_PROBE ')) == 302
                assert int(material_last('TCONSTRUCT_MATERIAL_TAG_COUNT_PROBE ')) == 18
                assert material_last('TCONSTRUCT_MATERIAL_WOOD_PROBE ') == '89e9d6af86e89ee4fb6079aa540323104c4afbe11784f5dca9c935ea7381b453'
            if a.choices_test:
                empty = [line.split('TCONSTRUCT_CHOICES_EMPTY ', 1)[1] for line in lines if 'TCONSTRUCT_CHOICES_EMPTY ' in line]
                wood = [line.split('TCONSTRUCT_CHOICES_WOOD ', 1)[1] for line in lines if 'TCONSTRUCT_CHOICES_WOOD ' in line]
                assert empty and empty[-1] == '28 tconstruct:arrow_head', empty
                if choice_layer == 'b':
                    expected_choices = 'tconstruct:pick_head:1,tconstruct:tool_binding:1,tconstruct:tool_handle:1'
                else:
                    cost = 3 if choice_layer == 'a' else 2
                    expected_choices = f'tconstruct:tool_binding:1,tconstruct:tool_handle:1,tconstruct:pick_head:{cost}'
                assert wood and wood[-1] == expected_choices, (wood, expected_choices)
            if a.lookup_test:
                flint = [line.split('TCONSTRUCT_LOOKUP_FLINT ', 1)[1] for line in lines if 'TCONSTRUCT_LOOKUP_FLINT ' in line]
                oak = [line.split('TCONSTRUCT_LOOKUP_OAK_LOG ', 1)[1] for line in lines if 'TCONSTRUCT_LOOKUP_OAK_LOG ' in line]
                assert flint and flint[-1] == 'tconstruct:tools/materials/flint/flint tconstruct:flint 1 1 0', flint
                assert oak and oak[-1] == 'tconstruct:tools/materials/wood/logs/oak tconstruct:wood 4 1 3', oak
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
    if a.pattern_atlas_test:
        first.call('command', 'tp @s 0.5 71 0.5')
        first.call('command', 'setblock 0 71 2 tconstruct:part_builder')
        time.sleep(1)
        first.call('interact', '0', '71', '2')
        first.call('wait', '5')
        lines = (first.root / 'game.log').read_text().splitlines()
        patterns = [line.split('TCONSTRUCT_CLIENT_PATTERN_ATLAS ', 1)[1] for line in lines if 'TCONSTRUCT_CLIENT_PATTERN_ATLAS ' in line]
        assert patterns and int(patterns[-1]) == 28, patterns
        first.call('close')
    probe(first, 'source-layer' if a.source_pack_test else 'missing', planks_expected=2 if a.vanilla_pack_test else None, forge_expected=1 if a.forge_pack_test else None)
    if a.source_pack_test:
        first.call('command', 'datapack disable vanilla')
        probe(first, 'source-layer', planks_expected=1 if a.vanilla_pack_test else None, forge_expected=1 if a.forge_pack_test else None, vanilla_enabled=False)
    pack(first, 'a', 'world-one-a')
    probe(first, 'world-one-a', 'source-layer|world-one-a' if a.source_pack_test else None, vanilla_enabled=not a.source_pack_test, choice_layer='a')
    pack(first, 'b', 'world-one-b')
    probe(first, 'world-one-b', 'source-layer|world-one-a|world-one-b' if a.source_pack_test else 'world-one-a|world-one-b', vanilla_enabled=not a.source_pack_test, choice_layer='b')
    first.call('command', 'datapack disable "file/b"')
    probe(first, 'world-one-a', 'source-layer|world-one-a' if a.source_pack_test else None, vanilla_enabled=not a.source_pack_test, choice_layer='a')
    second = Game(a, run, 'Second', in_frame=a.in_frame); games.append(second); second.ready()
    pack(second, 'a', 'world-two-a')
    probe(second, 'world-two-a', 'source-layer|world-two-a' if a.source_pack_test else None, choice_layer='a')
    probe(first, 'world-one-a', 'source-layer|world-one-a' if a.source_pack_test else None, vanilla_enabled=not a.source_pack_test, choice_layer='a')
    if a.world_config_test:
        config_path = first.root / 'config/tconstruct-common.toml'
        config_path.parent.mkdir(exist_ok=True)
        config_path.write_text('[debug]\nforceIntegrationMaterials = true\n')
        probe(first, 'world-one-a', 'source-layer|world-one-a' if a.source_pack_test else None, vanilla_enabled=not a.source_pack_test, choice_layer='a', config_expected='unsupported')
        probe(second, 'world-two-a', 'source-layer|world-two-a' if a.source_pack_test else None, choice_layer='a')
    for game in games: game.stop()
    print('PASS world separation, reload, priority, disable', flush=True)
finally:
    for game in games: game.cleanup()

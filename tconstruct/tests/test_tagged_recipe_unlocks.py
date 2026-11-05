"""Source tagged criteria, bounded world membership, and native recipe mapping."""
import copy
import hashlib
import json
from pathlib import Path
import re
import sys
import tempfile
import unittest
from zipfile import ZipFile

MOD = Path(__file__).resolve().parents[1]
DATA = MOD / 'content/data/tconstruct'
MIXINS = MOD / 'mixins/tconstruct'
FIXTURES = MOD / 'tests/fixtures/tagged_recipe_unlocks'
CASES = {
    'pattern': ('misc', 'tables/pattern', 'has_item', 'forge:rods/wooden'),
    'copper_can': ('misc', 'smeltery/copper_can', 'has_item', 'forge:ingots/copper'),
    'gold_bars': ('decorations', 'common/gold_bars', 'has_ingot', 'forge:ingots/gold'),
    'survival_seared_duct': ('redstone', 'smeltery/seared/duct', 'has_item', 'forge:ingots/gold'),
    'survival_scorched_duct': ('redstone', 'smeltery/scorched/duct', 'has_item', 'forge:ingots/gold'),
    'seared_fluid_cannon': ('redstone', 'smeltery/seared/fluid_cannon', 'has_item', 'forge:ingots/copper'),
}
for wood in ('greenheart', 'skyroot', 'bloodshroom', 'enderbark'):
    CASES[wood + '_planks'] = ('building_blocks', f'world/wood/{wood}/planks', 'has_log', f'tconstruct:{wood}_logs')
for slime in ('earth', 'sky', 'ichor', 'ender'):
    CASES[slime + '_congealed_slime'] = ('building_blocks', f'common/slime/{slime}/congealed', 'has_item', f'forge:slimeball/{slime}')
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class TaggedRecipeUnlocks(unittest.TestCase):
    def test_source_tags_or_requirements_and_native_recipe_rewards(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), set(CASES))
        self.assertEqual(len(manifest), 14)
        for name, (group, source, criterion, tag) in CASES.items():
            with self.subTest(recipe=name):
                raw = (FIXTURES / (name + '.json')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), manifest[name]['sha256'])
                self.assertEqual(manifest[name]['source'], f'{group}/{source}.json')
                original = json.loads(raw)
                self.assertEqual(original['criteria'][criterion]['conditions']['items'], [{'tag': tag}])
                expected = copy.deepcopy(original)
                expected['criteria'][criterion]['conditions']['items'] = [{'items': '#' + tag}]
                expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:' + name}
                expected['rewards']['recipes'] = ['tconstruct:' + name]
                native = json.loads((DATA / f'advancement/recipes/{group}/{name}.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['requirements'], [[criterion, 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)

    def test_only_declared_acquisition_tags_share_the_snapshot_bridge(self):
        helper = (MIXINS / 'source-advancement-tag.rs').read_text()
        tags = set(re.findall(r'"((?:forge|tconstruct):[^"]+)"', helper))
        self.assertEqual(tags, {case[3] for case in CASES.values()} |
                         {'tconstruct:modifiable/multipart', 'tconstruct:seared_blocks', 'tconstruct:anvil_metal', 'forge:glass/colorless', 'forge:slimeballs', 'tconstruct:casts/single_use/blank', 'tconstruct:casts/gold', 'tconstruct:parts'})
        for path in ('source-advancement-tag-index.rs', 'source-advancement-tag-predicate.rs'):
            source = (MIXINS / path).read_text()
            self.assertIn('tconstruct_source_advancement_tag::managed(tag)', source)
            self.assertIn('tconstruct_source_advancement_tag::members(game, tag)', source)
        self.assertIn('registry == "item"', (MIXINS / 'source-advancement-tag-predicate.rs').read_text())
        self.assertIn('items.item_tags.get(tag)', (MIXINS / 'source-advancement-tag-index.rs').read_text())

    def test_rewarded_native_recipe_shapes_and_results_match_source(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as source:
            for name, (_, source_path, _, _) in CASES.items():
                with self.subTest(recipe=name):
                    native = json.loads((DATA / f'recipe/{name}.json').read_text())
                    original = json.loads(source.read(f'data/tconstruct/recipes/{source_path}.json'))
                    self.assertEqual(native['type'], original['type'])
                    self.assertEqual(native.get('pattern'), original.get('pattern'))
                    self.assertEqual(native['result']['id'], original['result']['item'])
                    self.assertEqual(native['result'].get('count', 1), original['result'].get('count', 1))

    def test_package_contains_all_fourteen_unlocks_and_native_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name, (group, _, _, _) in CASES.items():
                    for relative in (f'advancement/recipes/{group}/{name}.json', f'recipe/{name}.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()

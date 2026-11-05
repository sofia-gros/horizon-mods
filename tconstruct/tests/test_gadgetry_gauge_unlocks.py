"""Source tagged discovery for the existing Gadgetry book and Copper Gauge."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

MOD = Path(__file__).resolve().parents[1]
DATA = MOD / 'content/data/tconstruct'
FIXTURES = MOD / 'tests/fixtures/gadgetry_gauge_unlocks'
CASES = {
    'tinkers_gadgetry': ('misc', 'common/tinkers_gadgetry', 'forge:slimeball/sky'),
    'copper_gauge': ('redstone', 'smeltery/seared/gauge', 'forge:ingots/copper'),
}
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class GadgetryGaugeUnlocks(unittest.TestCase):
    def source(self, name):
        raw = (FIXTURES / (name + '.json')).read_bytes()
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(hashlib.sha256(raw).hexdigest(), manifest[name]['sha256'])
        return json.loads(raw)

    def test_source_tag_criteria_or_requirements_and_single_native_rewards(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {*CASES, 'copper_gauge_recipe'})
        for name, (group, source_path, tag) in CASES.items():
            with self.subTest(name=name):
                expected = self.source(name)
                self.assertEqual(manifest[name]['source'], f'advancements/recipes/{group}/{source_path}.json')
                self.assertEqual(expected['criteria']['has_item']['conditions'], {'items': [{'tag': tag}]})
                self.assertEqual(expected['criteria']['has_the_recipe']['conditions'], {'recipe': 'tconstruct:' + source_path})
                expected['criteria']['has_item']['conditions'] = {'items': [{'items': '#' + tag}]}
                expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:' + name}
                expected['rewards']['recipes'] = ['tconstruct:' + name]
                native = json.loads((DATA / f'advancement/recipes/{group}/{name}.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['requirements'], [['has_item', 'has_the_recipe']])
                self.assertEqual(native['parent'], 'minecraft:recipes/root')
                self.assertNotIn('display', native)
                self.assertIn('"' + tag + '"', (MOD / 'mixins/tconstruct/source-advancement-tag.rs').read_text())

    def test_existing_crafting_recipes_counts_and_registrations(self):
        with ZipFile(MOD / 'content/reference/tconstruct-recipes.zip') as archive:
            source = json.loads(archive.read('data/tconstruct/recipes/common/tinkers_gadgetry.json'))
        native = json.loads((DATA / 'recipe/tinkers_gadgetry.json').read_text())
        self.assertEqual(native['type'], source['type'])
        self.assertEqual(native['ingredients'], [part.get('item') or '#' + part['tag'] for part in source['ingredients']])
        self.assertEqual(native['result'], {'id': source['result']['item'], 'count': source['result'].get('count', 1)})
        self.assertEqual(json.loads((DATA / 'horizon/item/tinkers_gadgetry.json').read_text())['max_stack_size'], 1)
        source = self.source('copper_gauge_recipe')
        native = json.loads((DATA / 'recipe/copper_gauge.json').read_text())
        self.assertEqual(native['type'], source['type'])
        self.assertEqual(native['pattern'], source['pattern'])
        self.assertEqual(native['key'], {key: '#' + value['tag'] for key, value in source['key'].items()})
        self.assertEqual(native['result'], {'id': source['result']['item'], 'count': source['result']['count']})
        self.assertTrue((DATA / 'horizon/block/copper_gauge.json').is_file())
        book = json.loads((DATA / 'advancement/world/tinkers_gadgetry.json').read_text())
        self.assertEqual(book['criteria']['crafted_book']['conditions']['items'], [{'items': ['tconstruct:tinkers_gadgetry']}])
        self.assertNotIn('rewards', book)

    def test_package_contains_both_unlocks_and_recipes(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name, (group, _, _) in CASES.items():
                    for relative in (f'advancement/recipes/{group}/{name}.json', f'recipe/{name}.json'):
                        self.assertEqual(archive.read('data/tconstruct/' + relative), (DATA / relative).read_bytes())


if __name__ == '__main__':
    unittest.main()

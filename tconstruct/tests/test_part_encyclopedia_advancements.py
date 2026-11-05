"""Pinned part/book milestones and their existing acquisition resources."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
FIXTURES = ROOT / 'tests/fixtures/part_encyclopedia'
CASES = {'make_part': ('tools', 'crafted_part', 'tconstruct:pick_head'),
         'encyclopedia': ('foundry', 'crafted_book', 'tconstruct:encyclopedia')}
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class PartEncyclopediaAdvancements(unittest.TestCase):
    def test_exact_source_criteria_display_and_parents(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), set(CASES))
        for name, (group, criterion, icon) in CASES.items():
            with self.subTest(advancement=name):
                raw = (FIXTURES / (name + '.json')).read_bytes()
                self.assertEqual(manifest[name], {'source': f'{group}/{name}.json', 'sha256': hashlib.sha256(raw).hexdigest()})
                original = json.loads(raw)
                expected = copy.deepcopy(original)
                if name == 'make_part':
                    self.assertEqual(original['criteria'][criterion]['conditions']['items'], [{'tag': 'tconstruct:parts'}])
                    self.assertEqual(original['display']['icon']['nbt'], '{Material:"tconstruct:wood"}')
                    expected['criteria'][criterion]['conditions']['items'] = [{'items': '#tconstruct:parts'}]
                expected['display']['icon'] = {'id': icon}
                native = json.loads((DATA / f'advancement/{group}/{name}.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['criteria'][criterion]['trigger'], 'minecraft:inventory_changed')
                self.assertEqual(native['requirements'], [[criterion]])
                self.assertTrue(native['display']['show_toast'])
                self.assertFalse(native['display']['hidden'])
                self.assertEqual(native['display']['announce_to_chat'], name == 'encyclopedia')
                self.assertEqual(native['display']['frame'], 'goal' if name == 'encyclopedia' else 'task')
                self.assertNotIn('rewards', native)
                parent = native['parent'].removeprefix('tconstruct:')
                self.assertTrue((DATA / f'advancement/{parent}.json').is_file())

    def test_existing_book_casting_route_and_part_snapshot_bridge(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            recipe = json.loads(archive.read('data/tconstruct/recipes/common/encyclopedia.json'))
        self.assertEqual(recipe, {'type': 'tconstruct:casting_table',
            'cast': {'item': 'minecraft:book'}, 'cast_consumed': True,
            'cooling_time': 57, 'fluid': {'amount': 90, 'tag': 'forge:molten_gold'},
            'result': 'tconstruct:encyclopedia'})
        self.assertTrue((DATA / 'horizon/item/encyclopedia.json').is_file())
        bridge = (ROOT / 'mixins/tconstruct/source-advancement-tag.rs').read_text()
        self.assertIn('"tconstruct:parts"', bridge)
        tags = ROOT / 'content/reference/tconstruct-tags/data/tconstruct/tags/items'
        parts = json.loads((tags / 'parts.json').read_text())['values']
        self.assertIn('#tconstruct:parts/bartered', parts)
        self.assertIn('tconstruct:pick_head', json.loads((tags / 'parts/bartered.json').read_text())['values'])

    def test_package_retains_goals_icons_and_encyclopedia_casting_recipe(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name, (group, _, icon) in CASES.items():
                    path = f'advancement/{group}/{name}.json'
                    self.assertEqual(archive.read('data/tconstruct/' + path), (DATA / path).read_bytes())
                    self.assertIn('data/tconstruct/horizon/item/' + icon.split(':', 1)[1] + '.json', archive.namelist())
                source_path = 'assets/tconstruct/source-packs/tconstruct-original-pack/data/tconstruct/recipes/common/encyclopedia.json'
                self.assertEqual(json.loads(archive.read(source_path))['result'], 'tconstruct:encyclopedia')


if __name__ == '__main__':
    unittest.main()

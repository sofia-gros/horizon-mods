"""Slime Collector's five source acquisition criteria and existing ancestry."""
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
FIXTURES = ROOT / 'tests/fixtures/slime_collector'
PATH = 'advancement/world/slime_collector.json'
SLIMES = ('earth', 'sky', 'ichor', 'ender')
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class SlimeCollectorAdvancement(unittest.TestCase):
    def test_exact_source_translation_and_all_five_required_criteria(self):
        raw = (FIXTURES / 'slime_collector.json').read_bytes()
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(manifest, {'slime_collector': {
            'source': 'world/slime_collector.json', 'sha256': hashlib.sha256(raw).hexdigest()}})
        source = json.loads(raw)
        expected = copy.deepcopy(source)
        bridge = (ROOT / 'mixins/tconstruct/source-advancement-tag.rs').read_text()
        for slime in SLIMES:
            tag = 'forge:slimeball/' + slime
            self.assertEqual(source['criteria'][slime], {'trigger': 'minecraft:inventory_changed',
                'conditions': {'items': [{'tag': tag}]}})
            expected['criteria'][slime]['conditions']['items'] = [{'items': '#' + tag}]
            self.assertIn('"' + tag + '"', bridge)
        expected['display']['icon'] = {'id': 'tconstruct:ichor_slime_ball'}
        native = json.loads((DATA / PATH).read_text())
        self.assertEqual(native, expected)
        self.assertEqual(set(native['criteria']), {*SLIMES, 'magma_cream'})
        self.assertEqual(native['criteria']['magma_cream'], {
            'trigger': 'minecraft:inventory_changed',
            'conditions': {'items': [{'items': ['minecraft:magma_cream']}]}})
        self.assertEqual(native['requirements'], [[name] for name in (*SLIMES, 'magma_cream')])
        self.assertEqual(native['display']['frame'], 'task')
        self.assertTrue(native['display']['show_toast'])
        self.assertFalse(native['display']['announce_to_chat'])
        self.assertFalse(native['display']['hidden'])
        self.assertNotIn('rewards', native)

    def test_existing_gadgetry_parent_connects_to_root(self):
        native = json.loads((DATA / PATH).read_text())
        self.assertEqual(native['parent'], 'tconstruct:world/tinkers_gadgetry')
        parent = json.loads((DATA / 'advancement/world/tinkers_gadgetry.json').read_text())
        self.assertEqual(parent['parent'], 'tconstruct:tools/materials_and_you')
        root = json.loads((DATA / 'advancement/tools/materials_and_you.json').read_text())
        self.assertNotIn('parent', root)

    def test_package_contains_goal_translations_and_declared_colored_slimeballs(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                self.assertEqual(archive.read('data/tconstruct/' + PATH), (DATA / PATH).read_bytes())
                language = json.loads(archive.read('assets/tconstruct/lang/en_us.json'))
                for field in ('title', 'description'):
                    self.assertIn('advancements.tconstruct.world.slime_collector.' + field, language)
                for slime in ('sky', 'ichor', 'ender'):
                    self.assertIn(f'data/tconstruct/horizon/item/{slime}_slime_ball.json', archive.namelist())


if __name__ == '__main__':
    unittest.main()

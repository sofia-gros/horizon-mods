"""Source Material Master requirements and native material-predicate wiring."""
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
MIXINS = ROOT / 'mixins/tconstruct'
FIXTURES = ROOT / 'tests/fixtures/material_master'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class MaterialMasterAdvancement(unittest.TestCase):
    def test_pinned_sources_and_exact_goal_adaptation(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'material_master.json', 'HasMaterialPredicate.java',
            'MaterialPredicateField.java', 'MaterialPredicate.java', 'MaterialVariantId.java'})
        for name, entry in manifest.items():
            self.assertEqual(hashlib.sha256((FIXTURES / name).read_bytes()).hexdigest(), entry['sha256'])
            self.assertTrue(entry['source'].endswith('/' + name))
        expected = copy.deepcopy(json.loads((FIXTURES / 'material_master.json').read_text()))
        expected['display']['icon'] = {'id': 'tconstruct:manyullyn_ingot'}
        actual = json.loads((DATA / 'advancement/tools/material_master.json').read_text())
        self.assertEqual(actual, expected)
        self.assertEqual(actual['parent'], 'tconstruct:tools/netherite_tier')
        self.assertEqual(len(actual['criteria']), 38)
        self.assertEqual(len(actual['requirements']), 38)
        self.assertEqual({tuple(row) for row in actual['requirements']},
                         {(name,) for name in actual['criteria']})
        for name, criterion in actual['criteria'].items():
            self.assertEqual(criterion, {'trigger': 'minecraft:inventory_changed',
                'conditions': {'items': [{'type': 'tconstruct:tool_stack', 'predicate': {
                    'type': 'tconstruct:has_material', 'material': 'tconstruct:' + name}}]}})
        self.assertNotIn('rewards', actual)

    def test_saved_materials_not_raw_items_or_upgrade_levels(self):
        helper = (MIXINS / 'source-advancement-tool-predicate.rs').read_text()
        self.assertIn('data.get("tic_materials")', helper)
        self.assertIn('w.in_tag("item", "tconstruct:modifiable", &it.id)', helper)
        self.assertIn('slot != index as usize', helper)
        self.assertIn('Some("default") => actual.1.is_none()', helper)
        self.assertIn('let Ok(id) = id.to_string()', helper)
        self.assertIn('predicate.as_str() != Some("tconstruct:has_upgrades")', helper)
        self.assertNotIn('trigger(', helper)
        self.assertIn('index >= 0', (FIXTURES / 'HasMaterialPredicate.java').read_text())
        self.assertIn('new MaterialIdPredicate(material.getId())',
                      (FIXTURES / 'MaterialPredicate.java').read_text())
        fixture = (ROOT / 'tests/modified-advancement/src/lib.rs').read_text()
        for test in ('material_base_ids_accept_variants', 'material_indexes_are_exact',
                     'material_match_keeps_item_tag', 'malformed_material_lists'):
            self.assertIn(test, fixture)

    def test_package_keeps_all_requirements_and_source_display_text(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                path = 'data/tconstruct/advancement/tools/material_master.json'
                self.assertEqual(archive.read(path), (ROOT / 'content' / path).read_bytes())
                language = json.loads(archive.read('assets/tconstruct/lang/en_us.json'))
                for field in ('title', 'description'):
                    self.assertIn('advancements.tconstruct.tools.material_master.' + field, language)


if __name__ == '__main__':
    unittest.main()

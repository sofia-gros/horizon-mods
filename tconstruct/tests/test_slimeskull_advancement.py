"""All source Slimeskull requirements retain item-plus-material conjunctions."""
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
FIXTURES = ROOT / 'tests/fixtures/slimeskull'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class SlimeskullAdvancement(unittest.TestCase):
    def test_exact_source_requirements_and_explicit_icon_adaptation(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'slimeskull.json', 'ToolStackPredicate.java'})
        for name, entry in manifest.items():
            self.assertEqual(hashlib.sha256((FIXTURES / name).read_bytes()).hexdigest(), entry['sha256'])
            self.assertTrue(entry['source'].endswith('/' + name))
        expected = copy.deepcopy(json.loads((FIXTURES / 'slimeskull.json').read_text()))
        self.assertIn('nbt', expected['display']['icon'])
        expected['display']['icon'] = {'id': 'tconstruct:slime_helmet'}
        goal = json.loads((DATA / 'advancement/world/slimeskull.json').read_text())
        self.assertEqual(goal, expected)
        self.assertEqual(len(goal['criteria']), 16)
        self.assertEqual(len(goal['requirements']), 16)
        self.assertEqual({tuple(row) for row in goal['requirements']}, {(name,) for name in goal['criteria']})
        self.assertEqual(goal['parent'], 'tconstruct:world/slimesuit')
        self.assertTrue((DATA / 'advancement/world/slimesuit.json').is_file())
        self.assertNotIn('rewards', goal)
        for name, criterion in goal['criteria'].items():
            self.assertEqual(criterion, {'trigger': 'minecraft:inventory_changed', 'conditions': {'items': [{
                'type': 'tconstruct:tool_stack', 'predicate': {'type': 'mantle:and', 'predicates': [
                    {'type': 'mantle:set', 'items': ['tconstruct:slime_helmet']},
                    {'type': 'tconstruct:has_material', 'index': 0, 'material': 'tconstruct:' + name}
                ]}}]}})

    def test_predicate_composition_uses_the_existing_production_helper(self):
        helper = (ROOT / 'mixins/tconstruct/source-advancement-tool-predicate.rs').read_text()
        self.assertIn('children.iter().all(|child| matches(child, it, depth + 1))', helper)
        self.assertIn('items.iter().all(|item| item.as_str().is_some_and(valid_id))', helper)
        self.assertIn('w.in_tag("item", "tconstruct:modifiable", &it.id)', helper)
        self.assertIn('if depth > 8', helper)
        self.assertIn('children.len() <= 32', helper)
        self.assertIn('items.len() <= 64', helper)
        fixture = (ROOT / 'tests/modified-advancement/src/lib.rs').read_text()
        self.assertIn('slimeskull_needs_the_helmet_and_its_first_material', fixture)
        self.assertIn('conjunctions_reject_empty_unknown_and_malformed_children', fixture)

    def test_content_package_keeps_goal_and_source_display_text(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                path = 'data/tconstruct/advancement/world/slimeskull.json'
                self.assertEqual(archive.read(path), (ROOT / 'content' / path).read_bytes())
                language = json.loads(archive.read('assets/tconstruct/lang/en_us.json'))
                for field in ('title', 'description'):
                    self.assertIn('advancements.tconstruct.world.slimeskull.' + field, language)


if __name__ == '__main__':
    unittest.main()

"""Source Modified Tools criterion, bounded metadata predicate, and package wiring."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import tomllib
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
MIXINS = ROOT / 'mixins/tconstruct'
FIXTURES = ROOT / 'tests/fixtures/modified_tool'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class ModifiedToolAdvancement(unittest.TestCase):
    def test_pinned_predicate_sources_and_exact_goal_adaptation(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'modified.json', 'ToolStackItemPredicate.java', 'ToolContextPredicate.java'})
        for name, entry in manifest.items():
            self.assertEqual(hashlib.sha256((FIXTURES / name).read_bytes()).hexdigest(), entry['sha256'])
            self.assertTrue(entry['source'].endswith('/' + name))
        expected = copy.deepcopy(json.loads((FIXTURES / 'modified.json').read_text()))
        expected['display']['icon'] = {'id': 'minecraft:redstone'}
        native = json.loads((DATA / 'advancement/tools/modified.json').read_text())
        self.assertEqual(native, expected)
        self.assertEqual(native['criteria'], {'crafted_tool': {
            'trigger': 'minecraft:inventory_changed', 'conditions': {'items': [{
                'type': 'tconstruct:tool_stack', 'predicate': 'tconstruct:has_upgrades'}]}}})
        self.assertEqual(native['requirements'], [['crafted_tool']])
        self.assertNotIn('rewards', native)
        self.assertIn('stack.is(Items.MODIFIABLE)', (FIXTURES / 'ToolStackItemPredicate.java').read_text())
        self.assertIn('!tool.getUpgrades().isEmpty()', (FIXTURES / 'ToolContextPredicate.java').read_text())

    def test_source_parent_chain_is_present(self):
        current = 'tconstruct:tools/modified'
        seen = set()
        while current:
            self.assertNotIn(current, seen)
            seen.add(current)
            namespace, path = current.split(':', 1)
            self.assertEqual(namespace, 'tconstruct')
            goal = json.loads((DATA / 'advancement' / (path + '.json')).read_text())
            current = goal.get('parent')
        self.assertIn('tconstruct:tools/tinker_tool', seen)
        self.assertIn('tconstruct:tools/materials_and_you', seen)

    def test_head_predicate_uses_metadata_and_shared_world_tag_bridge(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        entries = [entry for entry in manifest['injection'] if entry['id'] == 'source-advancement-tool-predicate']
        self.assertEqual(len(entries), 1)
        entry = entries[0]
        self.assertEqual(entry['target'], 'horizon_client::advancements::item')
        self.assertEqual(entry['kind'], 'HEAD')
        self.assertTrue(entry['cancellable'])
        self.assertEqual(entry['helper'], 'source-advancement-tool-predicate.rs')
        self.assertEqual({binding['name']: binding['value'] for binding in entry['bindings']},
                         {'v': 'v', 'it': 'it', 'w': 'w'})
        helper = (MIXINS / entry['helper']).read_text()
        self.assertIn('it.custom_data.as_ref()', helper)
        self.assertIn('data.get("tic_upgrades")', helper)
        self.assertIn('w.in_tag("item", "tconstruct:modifiable", &it.id)', helper)
        self.assertIn('"tconstruct:modifiable",', (MIXINS / 'source-advancement-tag.rs').read_text())
        self.assertNotIn('trigger(', helper)
        fixture = (ROOT / 'tests/modified-advancement/src/lib.rs').read_text()
        self.assertIn('include!("../../../mixins/tconstruct/source-advancement-tool-predicate.rs")', fixture)
        self.assertIn('unsupported_tool_predicates_do_not_fall_through', fixture)

    def test_content_package_contains_goal_and_source_translations(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                path = 'data/tconstruct/advancement/tools/modified.json'
                self.assertEqual(archive.read(path), (ROOT / 'content' / path).read_bytes())
                language = json.loads(archive.read('assets/tconstruct/lang/en_us.json'))
                for field in ('title', 'description'):
                    self.assertIn('advancements.tconstruct.tools.modified.' + field, language)


if __name__ == '__main__':
    unittest.main()

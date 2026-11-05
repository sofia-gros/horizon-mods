"""Source installed-upgrade goals retain same-tool and cumulative requirements."""
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
FIXTURES = ROOT / 'tests/fixtures/installed_modifier'
GOALS = {'upgrade_slots': 'tools/upgrade_slots', 'abilities': 'smeltery/abilities'}
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class InstalledModifierAdvancements(unittest.TestCase):
    def test_pinned_sources_and_exact_icon_only_adaptation(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'upgrade_slots.json', 'abilities.json',
            'HasModifierPredicate.java', 'SingleModifierPredicate.java'})
        for name, entry in manifest.items():
            self.assertEqual(hashlib.sha256((FIXTURES / name).read_bytes()).hexdigest(), entry['sha256'])
            self.assertTrue(entry['source'].endswith('/' + name))
        for name, path in GOALS.items():
            expected = copy.deepcopy(json.loads((FIXTURES / (name + '.json')).read_text()))
            expected['display']['icon'] = {'id': expected['display']['icon']['item']}
            goal = json.loads((DATA / 'advancement' / (path + '.json')).read_text())
            self.assertEqual(goal, expected)
            self.assertNotIn('rewards', goal)

    def test_five_upgrade_slots_are_required_on_one_tool(self):
        goal = json.loads((DATA / 'advancement/tools/upgrade_slots.json').read_text())
        self.assertEqual(goal['requirements'], [['has_modified']])
        self.assertEqual(set(goal['criteria']), {'has_modified'})
        criterion = goal['criteria']['has_modified']
        self.assertEqual(criterion['trigger'], 'minecraft:inventory_changed')
        self.assertEqual(len(criterion['conditions']['items']), 1)
        predicate = criterion['conditions']['items'][0]['predicate']
        self.assertEqual(predicate['type'], 'mantle:and')
        self.assertEqual(predicate['predicates'], [{'type': 'tconstruct:has_modifier', 'check': 'upgrades',
            'modifier': {'type': 'tconstruct:single', 'modifier': 'tconstruct:' + name}}
            for name in ('writable', 'recapitated', 'harmonious', 'forecast', 'gilded')])

    def test_all_57_ability_requirements_remain_independent(self):
        goal = json.loads((DATA / 'advancement/smeltery/abilities.json').read_text())
        self.assertEqual(len(goal['criteria']), 57)
        self.assertEqual(len(goal['requirements']), 57)
        self.assertEqual({tuple(row) for row in goal['requirements']}, {(name,) for name in goal['criteria']})
        for name, criterion in goal['criteria'].items():
            self.assertEqual(criterion, {'trigger': 'minecraft:inventory_changed', 'conditions': {'items': [{
                'type': 'tconstruct:tool_stack', 'predicate': {'type': 'tconstruct:has_modifier',
                    'check': 'upgrades', 'modifier': {'type': 'tconstruct:single', 'modifier': 'tconstruct:' + name}}
            }]}})

    def test_source_parent_chains_are_present(self):
        for path in GOALS.values():
            current = 'tconstruct:' + path
            seen = set()
            while current:
                self.assertNotIn(current, seen)
                seen.add(current)
                namespace, path = current.split(':', 1)
                self.assertEqual(namespace, 'tconstruct')
                current = json.loads((DATA / 'advancement' / (path + '.json')).read_text()).get('parent')
            self.assertIn('tconstruct:tools/materials_and_you', seen)

    def test_predicate_is_installed_presence_not_all_modifiers(self):
        helper = (ROOT / 'mixins/tconstruct/source-advancement-tool-predicate.rs').read_text()
        self.assertIn('predicate.get("level").is_some()', helper)
        self.assertIn('Some("upgrades")', helper)
        self.assertIn('Some("tconstruct:single")', helper)
        self.assertIn('has_upgrade(it, Some(id))', helper)
        self.assertIn('has_upgrade(it, None)', helper)
        self.assertIn('return tool.getUpgrades()', (FIXTURES / 'HasModifierPredicate.java').read_text())
        self.assertIn('return input.equals(modifier)', (FIXTURES / 'SingleModifierPredicate.java').read_text())

    def test_content_package_contains_both_goals_and_translations(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                language = json.loads(archive.read('assets/tconstruct/lang/en_us.json'))
                for path in GOALS.values():
                    packaged = 'data/tconstruct/advancement/' + path + '.json'
                    self.assertEqual(archive.read(packaged), (ROOT / 'content' / packaged).read_bytes())
                    for field in ('title', 'description'):
                        self.assertIn('advancements.tconstruct.' + path.replace('/', '.') + '.' + field, language)


if __name__ == '__main__':
    unittest.main()

"""Source Netherite Tier criterion, computed snapshot, and restored ancestry."""
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
FIXTURES = ROOT / 'tests/fixtures/netherite_tier'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class NetheriteTierAdvancement(unittest.TestCase):
    def test_source_goal_and_material_master_ancestry(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'netherite_tier.json', 'StatInSetPredicate.java'})
        for name, entry in manifest.items():
            self.assertEqual(hashlib.sha256((FIXTURES / name).read_bytes()).hexdigest(), entry['sha256'])
            self.assertTrue(entry['source'].endswith('/' + name))
        expected = copy.deepcopy(json.loads((FIXTURES / 'netherite_tier.json').read_text()))
        expected['display']['icon'] = {'id': 'minecraft:netherite_ingot'}
        goal = json.loads((DATA / 'advancement/tools/netherite_tier.json').read_text())
        self.assertEqual(goal, expected)
        self.assertEqual(goal['requirements'], [['harvest_level']])
        self.assertNotIn('rewards', goal)
        current = 'tconstruct:tools/material_master'
        seen = set()
        while current:
            self.assertNotIn(current, seen)
            seen.add(current)
            namespace, path = current.split(':', 1)
            self.assertEqual(namespace, 'tconstruct')
            current = json.loads((DATA / 'advancement' / (path + '.json')).read_text()).get('parent')
        self.assertIn('tconstruct:tools/netherite_tier', seen)
        self.assertIn('tconstruct:tools/tinker_tool', seen)
        self.assertIn('tconstruct:tools/materials_and_you', seen)

    def test_shared_assembly_snapshot_and_world_local_harvest_tag(self):
        assembly = (MIXINS / 'source-station-world.rs').read_text()
        self.assertIn('gem_tier_with_traits(', assembly)
        self.assertIn('tier.filter(|_| harvest)', assembly)
        self.assertIn('if tier == "wooden" { "wood" } else { tier }', assembly)
        self.assertIn('fields.remove(&"tic_harvest_tier".into())', assembly)
        self.assertIn('effective_tier_snapshot_tracks_material_upgrade_trait_and_rebuild', assembly)
        helper = (MIXINS / 'source-advancement-tool-predicate.rs').read_text()
        self.assertIn('data.get("tic_harvest_tier")', helper)
        self.assertIn('w.in_tag("item", "tconstruct:modifiable/harvest", &it.id)', helper)
        self.assertIn('Some("tconstruct:harvest_tier")', helper)
        self.assertIn('"tconstruct:modifiable/harvest",', (MIXINS / 'source-advancement-tag.rs').read_text())
        self.assertIn('values.contains(tool.getStats().get(stat))', (FIXTURES / 'StatInSetPredicate.java').read_text())

    def test_content_package_keeps_goal_and_source_display_text(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                path = 'data/tconstruct/advancement/tools/netherite_tier.json'
                self.assertEqual(archive.read(path), (ROOT / 'content' / path).read_bytes())
                language = json.loads(archive.read('assets/tconstruct/lang/en_us.json'))
                for field in ('title', 'description'):
                    self.assertIn('advancements.tconstruct.tools.netherite_tier.' + field, language)


if __name__ == '__main__':
    unittest.main()

"""Bounded source Foundry opening criterion and distinct controller type."""
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
FIXTURES = ROOT / 'tests/fixtures/foundry_opening'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class FoundryOpeningAdvancement(unittest.TestCase):
    def test_source_goal_and_alloyer_parent_are_not_substituted(self):
        raw = (FIXTURES / 'structure.json').read_bytes()
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(manifest, {'structure': {'source': 'foundry/structure.json',
            'sha256': hashlib.sha256(raw).hexdigest()}})
        expected = copy.deepcopy(json.loads(raw))
        expected['display']['icon'] = {'id': 'tconstruct:foundry_controller'}
        goal = json.loads((DATA / 'advancement/foundry/structure.json').read_text())
        self.assertEqual(goal, expected)
        self.assertEqual(goal['criteria'], {'open_foundry': {
            'trigger': 'tconstruct:block_container_opened', 'conditions': {'type': 'tconstruct:foundry'}}})
        self.assertEqual(goal['parent'], 'tconstruct:foundry/alloyer')
        self.assertEqual(goal['requirements'], [['open_foundry']])
        self.assertNotIn('rewards', goal)

    def test_distinct_types_reuse_success_only_local_and_guest_paths(self):
        helper = (MIXINS / 'source-container-advancement.rs').read_text()
        self.assertIn('"tconstruct:foundry_controller" => "tconstruct:foundry"', helper)
        self.assertIn('"tconstruct:smeltery_controller" => "tconstruct:smeltery"', helper)
        self.assertIn('conditions["type"].as_str() == Some(container_type)', helper)
        for path in ('source-machine-open.rs', 'guest-open.rs'):
            source = (MIXINS / path).read_text()
            self.assertIn('"tconstruct:smeltery_controller" | "tconstruct:foundry_controller"', source)
            self.assertIn('tconstruct_source_container_advancement::opened', source)
        guest = (MIXINS / 'guest-open.rs').read_text()
        self.assertIn('game.as_guest(host, conn, |actor|', guest)
        self.assertIn('menu.pos == Some(pos)', guest)
        local = (MIXINS / 'source-machine-open.rs').read_text()
        self.assertIn('game.lan_acting.is_none()', local)

    def test_package_contains_goal_controller_and_source_translations(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                path = 'data/tconstruct/advancement/foundry/structure.json'
                self.assertEqual(archive.read(path), (ROOT / 'content' / path).read_bytes())
                self.assertIn('data/tconstruct/horizon/block/foundry_controller.json', archive.namelist())
                language = json.loads(archive.read('assets/tconstruct/lang/en_us.json'))
                for field in ('title', 'description'):
                    self.assertIn('advancements.tconstruct.foundry.structure.' + field, language)


if __name__ == '__main__':
    unittest.main()

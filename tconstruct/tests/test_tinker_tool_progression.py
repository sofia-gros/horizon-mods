"""Tinker Tool ancestry and the narrow world-tag advancement bridge."""
import json
import sys
import zipfile
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class TinkerToolProgression(unittest.TestCase):
    def test_source_goal_preserves_parent_notifications_and_native_tag_predicate(self):
        path = ROOT / 'content/data/tconstruct/advancement/tools/tinker_tool.json'
        goal = json.loads(path.read_text())
        self.assertEqual(goal['parent'], 'tconstruct:tools/tinker_station')
        self.assertEqual(goal['criteria'], {'crafted_tool': {'conditions': {
            'items': [{'items': '#tconstruct:modifiable/multipart'}]}, 'trigger': 'minecraft:inventory_changed'}})
        self.assertEqual(goal['requirements'], [['crafted_tool']])
        self.assertEqual(goal['display']['icon'], {'id': 'tconstruct:pickaxe'})
        self.assertEqual(goal['display']['frame'], 'task')
        self.assertFalse(goal['display']['announce_to_chat'])
        self.assertTrue(goal['display']['show_toast'])
        self.assertFalse(goal['display']['hidden'])
        self.assertFalse(goal['sends_telemetry_event'])
        self.assertNotIn('rewards', goal)
        child = json.loads((path.parent / 'tool_smith.json').read_text())
        self.assertEqual(child['parent'], 'tconstruct:tools/tinker_tool')
        self.assertTrue((path.parent / 'tinker_station.json').is_file())

    def test_snapshot_refresh_invalidates_index_after_replacing_membership(self):
        source = (MIXINS / 'recipe-reload.rs').read_text()
        assignment = source.index('*game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags_mut() = resolved_tags;')
        invalidation = source.index('game.tconstruct_invalidate_advancement_inventory_index();')
        self.assertGreater(invalidation, assignment)
        manifest = (MIXINS / 'horizon-mixin.toml').read_text()
        for helper in ('source-advancement-tag-predicate.rs', 'source-advancement-tag-index.rs'):
            self.assertIn(helper, manifest)
        predicate = (MIXINS / 'source-advancement-tag-predicate.rs').read_text()
        self.assertIn('registry == "item" && tag == "tconstruct:modifiable/multipart"', predicate)

    def test_packaged_goal_and_required_tag_members_are_registered(self):
        with tempfile.TemporaryDirectory(prefix='tinker-tool-package-') as directory:
            path = Path(directory) / 'tconstruct.hmod'
            package(path)
            with zipfile.ZipFile(path) as archive:
                relative = 'data/tconstruct/advancement/tools/tinker_tool.json'
                self.assertEqual(archive.read(relative), (ROOT / 'content' / relative).read_bytes())
                tag = json.loads((ROOT / 'content/reference/tconstruct-tags/data/tconstruct/tags/items/modifiable/multipart.json').read_text())
                for entry in tag['values']:
                    if not isinstance(entry, str):
                        continue  # the source's optional integration entries
                    namespace, name = entry.split(':')
                    self.assertIn(f'data/{namespace}/horizon/item/{name}.json', archive.namelist())

    @unittest.skipUnless(shutil.which('rustc'), 'Rust toolchain unavailable')
    def test_runtime_bridge_boundary_and_reload_fixture(self):
        source = (ROOT / 'tests/rust/tinker_tool_advancement.rs').read_text()
        for marker, file in (('HELPER', 'source-advancement-tag.rs'),
                             ('PREDICATE', 'source-advancement-tag-predicate.rs'),
                             ('INDEX', 'source-advancement-tag-index.rs')):
            source = source.replace('__' + marker + '__', str(MIXINS / file).replace('\\', '\\\\'))
        with tempfile.TemporaryDirectory(prefix='tinker-tool-advancement-') as directory:
            path = Path(directory) / 'fixture.rs'
            path.write_text(source)
            executable = Path(directory) / 'fixture'
            subprocess.run(['rustc', '--edition=2021', '--test', str(path), '-o', str(executable)], check=True)
            subprocess.run([str(executable)], check=True)


if __name__ == '__main__':
    unittest.main()

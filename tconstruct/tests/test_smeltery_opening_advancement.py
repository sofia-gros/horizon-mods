"""Source container-open goal and success-only local/guest wiring."""
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
FIXTURES = ROOT / 'tests/fixtures/smeltery_opening'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class SmelteryOpeningAdvancement(unittest.TestCase):
    def test_exact_source_goal_retains_custom_trigger_and_type(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {'structure.json', 'BlockContainerOpenedTrigger.java'})
        for name, entry in manifest.items():
            self.assertEqual(hashlib.sha256((FIXTURES / name).read_bytes()).hexdigest(), entry['sha256'])
        source = json.loads((FIXTURES / 'structure.json').read_text())
        expected = copy.deepcopy(source)
        expected['display']['icon'] = {'id': 'tconstruct:smeltery_controller'}
        goal = json.loads((DATA / 'advancement/smeltery/structure.json').read_text())
        self.assertEqual(goal, expected)
        self.assertEqual(goal['criteria'], {'open_smeltery': {
            'trigger': 'tconstruct:block_container_opened', 'conditions': {'type': 'tconstruct:smeltery'}}})
        self.assertEqual(goal['parent'], 'tconstruct:smeltery/mighty_smelting')
        self.assertEqual(goal['requirements'], [['open_smeltery']])
        self.assertNotIn('rewards', goal)

    def test_local_hook_follows_validation_and_skips_temporary_guest_contexts(self):
        source = (MIXINS / 'source-machine-open.rs').read_text()
        start = source.index('if matches!(name, "tconstruct:smeltery_controller"')
        end = source.index('if !matches!(name, "tconstruct:seared_melter"', start)
        branch = source[start:end]
        award = branch.index('tconstruct_source_container_advancement::opened(game, name)')
        for guard in ('::ensure(game, key, name)', '::detect(game, key).is_none()',
                      'state.slots.is_empty()', 'game.container = Some(key)', 'game.lan_acting.is_none()'):
            self.assertLess(branch.index(guard), award)

    def test_guest_hook_uses_actual_open_menu_and_guest_actor_only(self):
        source = (MIXINS / 'guest-open.rs').read_text()
        start = source.index('if matches!(&*machine, "tconstruct:smeltery_controller"')
        end = source.index('if matches!(&*machine, "tconstruct:seared_melter"', start)
        branch = source[start:end]
        award = branch.index('tconstruct_source_container_advancement::opened(actor, &machine)')
        for guard in ('::detect(game, pos).is_none()', 'state.slots.is_empty()',
                      'host.hub.open_container(', 'menu.pos == Some(pos)',
                      'menu.slots == horizon_client::lan::menu::Slots::Block(pos)',
                      'if opened {', 'game.as_guest(host, conn, |actor|'):
            self.assertLess(branch.index(guard), award)
        self.assertNotIn('::opened(game,', branch)

    def test_module_uses_native_player_predicate_and_advancement_pipeline(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        entry = next(value for value in manifest['module'] if value['id'] == 'source-container-advancement')
        self.assertEqual(entry['file'], 'crates/horizon-client/src/game.rs')
        self.assertEqual(entry['source'], 'source-container-advancement.rs')
        source = (MIXINS / entry['source']).read_text()
        for token in ('block != "tconstruct:smeltery_controller"', 'game.remote.is_some()',
                      'game.player_entity()', 'game.trigger("tconstruct:block_container_opened"',
                      'conditions["type"].as_str() == Some("tconstruct:smeltery")',
                      'adv::conditions(&conditions["player"], &context, world)'):
            self.assertIn(token, source)

    def test_package_connects_anvil_and_tool_forge_ancestry(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                goal_path = 'data/tconstruct/advancement/smeltery/structure.json'
                self.assertEqual(archive.read(goal_path), (ROOT / 'content' / goal_path).read_bytes())
                for child, parent in [('tool_forge', 'tinkers_anvil'), ('tinkers_anvil', 'structure'),
                                      ('structure', 'mighty_smelting'), ('mighty_smelting', 'melter')]:
                    node = json.loads(archive.read(f'data/tconstruct/advancement/smeltery/{child}.json'))
                    self.assertEqual(node['parent'], 'tconstruct:smeltery/' + parent)


if __name__ == '__main__':
    unittest.main()

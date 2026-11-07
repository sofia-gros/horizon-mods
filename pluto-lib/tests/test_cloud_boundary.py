"""Cloud construction stays explicit, staged and consumer-independent."""
from pathlib import Path
import importlib.util
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
NATIVE = ROOT / 'pluto-lib/mixins/pluto_native_creative'


class CloudBoundary(unittest.TestCase):
    def test_descriptor_uses_one_catalog_owner_and_no_game_or_source_policy(self):
        source = (NATIVE / 'cloud-descriptor.rs').read_text()
        self.assertIn('use horizon_client::pluto_potions;', source)
        for forbidden in ('horizon_client_core::potions', 'Game<', 'tconstruct:', 'impl Default', 'take_entity_id', 'breath_clouds', 'lan_acting'):
            self.assertNotIn(forbidden, source)
        self.assertIn('pub attribution: Attribution', source)
        self.assertIn('ConflictingFlags', source)
        self.assertIn('pub const MAX_EFFECTS: usize = 256;', source)

    def test_factory_is_side_effect_free_and_populates_new_native_fields(self):
        source = (NATIVE / 'cloud-factory.rs').read_text().split('#[cfg(test)]', 1)[0]
        for forbidden in ('Game<', 'game.', 'take_entity_id', 'breath_clouds', 'lan_acting', 'potion_owner(', 'Default::default'):
            self.assertNotIn(forbidden, source)
        for required in ('id: 0', 'victims: Vec::new()', 'settings: CloudSettings', 'source, owner, guest',
                         'duration_scale: d.timing.effect_duration_scale', 'wait_time: d.timing.wait_ticks',
                         'radius_on_use: d.radius_on_use', 'duration_on_use: d.timing.duration_on_use',
                         'Attribution::Unowned => (None, None)',
                         'Attribution::Player { guest_connection } => (Some(Who::Player), guest_connection)',
                         'Attribution::Mob { entity_id } => (Some(Who::Mob(entity_id)), None)'):
            self.assertIn(required, source)
        self.assertIn('native: Breath,', source)
        self.assertNotIn('pub native:', source)

    def test_exact_module_placement_has_no_consumer_activation(self):
        manifest = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text())
        modules = {m['id']: m for m in manifest['module']}
        self.assertEqual(modules['cloud-descriptor'], {'id':'cloud-descriptor',
            'file':'crates/horizon-client/src/client_lib.rs', 'name':'pluto_cloud',
            'source':'cloud-descriptor.rs', 'visibility':'pub'})
        self.assertEqual(modules['cloud-factory'], {'id':'cloud-factory',
            'file':'crates/horizon-client/src/game/potions.rs', 'name':'pluto_cloud_factory',
            'source':'cloud-factory.rs'})
        for path in (ROOT / 'tconstruct/mixins/tconstruct').glob('*.rs'):
            self.assertNotIn('pluto_cloud', path.read_text(), path.name)
        self.assertFalse(any('cloud' in hook['id'] for hook in manifest.get('injection', [])))

    def test_fixture_identity_is_specific_not_an_open_package_exception(self):
        path = ROOT / 'pluto-lib/tools/prepare_render_fixture.py'
        spec = importlib.util.spec_from_file_location('guards', path)
        guards = importlib.util.module_from_spec(spec); spec.loader.exec_module(guards)
        identity = ('pluto-cloud-descriptor', '0.0.0', None, None)
        fixture = {'name':identity[0], 'version':identity[1]}
        guards.check_identities({'package':[]}, {'package':[fixture]}, identity)
        for extra in ({'name':'other', 'version':'0.0.0'}, fixture | {'version':'0.0.1'}):
            with self.assertRaises(ValueError):
                guards.check_identities({'package':[]}, {'package':[fixture, extra]}, identity)


if __name__ == '__main__':
    unittest.main()

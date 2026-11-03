"""Guard the source-level invariants retained during the #354/main integration."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins' / 'tconstruct'

class MainIntegrationGuards(unittest.TestCase):
    def test_manifest_ids_are_unique(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        for kind in ('module', 'state', 'injection'):
            ids = [entry['id'] for entry in manifest.get(kind, [])]
            self.assertEqual(len(ids), len(set(ids)), kind)

    def test_overwield_keeps_post_cancellation_boundary(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        hook = next(entry for entry in manifest['injection'] if entry['id'] == 'source-overwield-harvest-start')
        self.assertEqual(hook['kind'], 'AFTER_CALL')
        self.assertEqual(hook['selector'], 'mod_api::dispatch_destroy_block')
        self.assertTrue(any(b['name'] == 'canceled' for b in hook['bindings']))

    def test_device_and_condition_modules_are_retained(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        sources = {entry['source'] for entry in manifest['module']}
        for name in ('source-conditions.rs', 'source-casting-tank.rs', 'source-fluid-cannon.rs', 'source-gauge.rs'):
            self.assertIn(name, sources)
            self.assertTrue((MIXINS / name).is_file())
        for name in ('seared_lantern', 'scorched_lantern', 'seared_casting_tank', 'seared_fluid_cannon', 'copper_gauge'):
            self.assertTrue((ROOT / 'content/data/tconstruct/horizon/block' / (name + '.json')).is_file())

    def test_chute_keeps_callback_snapshot_check(self):
        source = (MIXINS / 'source-chute-hopper.rs').read_text()
        callback = source.index('game.native_inventory_move_item(')
        recheck = source.index('if !chute_snapshots_match(', callback)
        commit = source.index('let mut next_hopper', callback)
        self.assertLess(callback, recheck)
        self.assertLess(recheck, commit)
        self.assertIn('fn cannon_transfer(', source)
        self.assertIn('fn casting_tank_transfer(', source)

if __name__ == '__main__':
    unittest.main()

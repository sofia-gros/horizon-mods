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

    def test_casting_hopper_intercepts_before_generic_transfer_and_rechecks(self):
        source = (MIXINS / 'source-chute-hopper.rs').read_text()
        for function in ('eject', 'suck'):
            body = source.split(f'pub(super) fn {function}(', 1)[1].split('\n}', 1)[0]
            self.assertIn('"tconstruct:seared_table" | "tconstruct:scorched_table" | "tconstruct:seared_basin"', body)
            self.assertIn('return Break(casting_transfer(', body)
        body = source.split('fn casting_transfer(', 1)[1].split('\n}\n', 1)[0]
        event = body.index('game.native_inventory_move_item(')
        snapshot = body.index('!= Some(&saved)', event)
        stage = body.index('state.hopper_transfer(', snapshot)
        write = body.index('tconstruct_source_melter_store::write(', stage)
        self.assertLess(event, snapshot)
        self.assertLess(snapshot, stage)
        self.assertLess(stage, write)
        self.assertIn('encode_moving_item(&item)', body)
        self.assertIn('pluto_stack_storage::encode_item_lossless as encode_moving_item', source)
        self.assertIn('pluto_inventory::select_candidate(candidates,', body)
        self.assertIn('game.container_changed(target)', body)

if __name__ == '__main__':
    unittest.main()

"""Generic storage mechanics must not absorb machine schemas or policy."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
NATIVE = ROOT / 'pluto-lib/mixins/pluto_native_creative'


class SharedNbtStorage(unittest.TestCase):
    def test_native_export_is_public_and_schema_free(self):
        manifest = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text())
        entry, = [item for item in manifest['module'] if item['id'] == 'nbt-storage']
        self.assertEqual((entry['name'], entry['source'], entry['visibility']),
                         ('pluto_nbt_storage', 'nbt-storage.rs', 'pub'))
        source = (NATIVE / entry['source']).read_text()
        self.assertIn('original: &Tag', source)
        self.assertNotIn('to_string_lossy', source)
        for policy in ['tconstruct:', 'FluidName', 'coolingTime', 'lastRedstone']:
            self.assertNotIn(policy, source)

    def test_machine_callers_keep_validation_and_ownership(self):
        mixins = ROOT / 'tconstruct/mixins/tconstruct'
        for filename in ['source-melter-state.rs', 'source-faucet-tank.rs', 'source-smeltery-nbt-preserve.rs']:
            self.assertIn('horizon_client::pluto_nbt_storage::patch_', (mixins / filename).read_text())
        state = (mixins / 'source-melter-state.rs').read_text()
        self.assertIn('id.to_string().ok()?', state)
        self.assertIn('"fluid", "fluid_tag", "amount"', state)
        self.assertIn('if !self.items_save_safe() { return None; }', state)
        self.assertIn('if unchanged { continue; }', state)
        smeltery = (mixins / 'source-smeltery-nbt-preserve.rs').read_text()
        self.assertIn('if lossy && old_tank != new_tank { return None; }', smeltery)
        self.assertIn('"capacity", "fluids", "FluidName", "Amount", "Tag"', smeltery)

    def test_native_fixture_requires_opaque_key_and_ordering_tests(self):
        runner = (ROOT / 'tconstruct/tests/run_inventory_transaction_guards.py').read_text()
        self.assertIn('unknown_utf16_keys_nested_values_and_float_bits_are_untouched', runner)
        self.assertIn('last_update_can_remove_a_new_field', runner)
        self.assertIn('malformed_original_does_not_consume_update_iterator', runner)


if __name__ == '__main__':
    unittest.main()

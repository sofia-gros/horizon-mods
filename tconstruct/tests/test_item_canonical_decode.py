"""Canonical saved-item decoding stays separate from consumer/snapshot policy."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ItemCanonicalDecode(unittest.TestCase):
    def test_shared_adapter_keeps_the_native_roundtrip_rule(self):
        source = (ROOT / 'pluto-lib/mixins/pluto_native_creative/stack-storage.rs').read_text()
        body = source.split('pub fn decode_item_canonical(', 1)[1].split('\n}', 1)[0]
        self.assertIn('let decoded = item_from_nbt(raw)?;', body)
        self.assertIn('(item_to_nbt(&decoded) == *raw).then_some(decoded)', body)
        for unrelated in ('same_item_snapshot', 'contains_retained_wire', 'count ==', 'tconstruct'):
            self.assertNotIn(unrelated, body)

    def test_exact_three_call_sites_keep_policy_local(self):
        source = ROOT / 'tconstruct/mixins/tconstruct'
        page = (source / 'source-smeltery-page.rs').read_text()
        state = (source / 'source-smeltery-state.rs').read_text()
        self.assertEqual(page.count('::decode_item_canonical('), 2)
        self.assertEqual(state.count('::decode_item_canonical('), 1)
        self.assertIn('decoded.count != 1 || !one.same_item_same_components(&decoded)', page)
        self.assertIn('matches!(raw_slot, Tag::Compound(map) if map.is_empty()) { continue; }', page)
        self.assertIn('pluto_stack_storage::encode_item_lossless(item)?', state)
        self.assertIn('raw_slots[index] != old', state)
        self.assertIn('raw.get("Items").is_some()', state)

    def test_inventory_serializer_has_one_native_lossless_guard(self):
        source = (ROOT / 'tconstruct/mixins/tconstruct/source-smeltery-state.rs').read_text()
        body = source.split('fn inventory_to_nbt_preserving(', 1)[1].split('fn from_nbt(', 1)[0]
        self.assertIn('pluto_stack_storage::encode_item_lossless(item)?', body)
        self.assertIn('pluto_stack_storage::decode_item_canonical(&encoded)?', body)
        self.assertNotIn('wire_patch()', body)
        self.assertNotIn('data.wire.patch()', body)
        self.assertNotIn('same_item_same_components', body)

    def test_fixture_uses_real_types_actual_api_and_host_lock(self):
        runner = (ROOT / 'tconstruct/tests/run_item_canonical_decode.py').read_text()
        self.assertIn("add_inventory_api(work / 'Cargo.toml', root)", runner)
        self.assertIn("lock_fixture_to_host(work / 'Cargo.toml', host, root)", runner)
        self.assertIn("'--locked', '--offline'", runner)
        fixture = (ROOT / 'tconstruct/tests/rust/item_canonical_decode.rs').read_text()
        for case in ('previous_single_insert', 'previous_inventory_patch',
                     'native_decode_matches_previous_tag_roundtrip_matrix',
                     'canonicality_is_not_saved_or_runtime_snapshot_identity'):
            self.assertIn(case, fixture)


if __name__ == '__main__':
    unittest.main()

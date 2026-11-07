"""Keep mechanical item identity in Pluto while machine policy stays local."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'tconstruct/mixins/tconstruct'


class SharedItemStorage(unittest.TestCase):
    def test_machine_save_predicates_delegate_to_real_public_adapter(self):
        for file, start, end in (
            ('source-melter-state.rs', 'pub(super) fn casting_item_save_safe(', '\n}\n'),
            ('source-smeltery-fuel.rs', 'pub(super) fn heater_stack_save_safe(', '\n}\n'),
            ('source-fluid-cannon.rs', 'pub(super) fn lossless(', '\n    }'),
            ('source-casting-tank.rs', 'pub(super) fn lossless(', '\n    }'),
        ):
            body = (SOURCE / file).read_text().split(start, 1)[1].split(end, 1)[0]
            self.assertIn('horizon_client::pluto_stack_storage::encode_item_lossless(', body)
            self.assertNotIn('item_from_nbt', body)
            self.assertNotIn('wire_patch()', body)

    def test_every_hopper_recheck_includes_opaque_identity(self):
        source = (SOURCE / 'source-chute-hopper.rs').read_text()
        self.assertNotIn('*slots == before', source)
        self.assertEqual(source.count('horizon_client::pluto_stack_storage::inventory_snapshot_matches('), 6)
        # The transferred slot still obeys the original save/transport policy.
        self.assertEqual(source.count('slots.get(index).and_then(Option::as_ref).is_none_or(|item| wire_safe(item))'), 3)
        for save in ('save_cannon(game,target,&cannon)', 'save_casting_tank(game, target, &state)'):
            self.assertIn(save, source)

    def test_native_adapter_keeps_save_eligibility_separate_from_snapshot(self):
        source = (ROOT / 'pluto-lib/mixins/pluto_native_creative/stack-storage.rs').read_text()
        save, snapshot = source.split('pub fn same_item_snapshot(', 1)
        self.assertIn('item.count == 0', save)
        self.assertIn('data.wire.patch().is_some()', save)
        self.assertIn('before.wire_patch() == after.wire_patch()', snapshot)
        self.assertIn('before.data().and_then(|data| data.wire.patch())', snapshot)
        self.assertNotIn('encode_item_lossless(', snapshot)

    def test_nested_wire_traversal_has_one_native_owner(self):
        source = (ROOT / 'pluto-lib/mixins/pluto_native_creative/stack-storage.rs').read_text()
        for field in ('container', 'charged', 'pot_decorations', 'bundle_contents'):
            self.assertIn(f'data.{field}.iter()', source)
        self.assertIn('pub fn contains_retained_wire(item: &ItemStack)', source)
        self.assertIn('item.count == 0 || contains_retained_wire(item)', source)
        self.assertIn('nested_stacks(before).zip(nested_stacks(after))', source)


if __name__ == '__main__':
    unittest.main()

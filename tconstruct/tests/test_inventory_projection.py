"""Projection is generic while layouts and authoritative write rules stay local."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class InventoryProjection(unittest.TestCase):
    def test_real_consumers_delegate_only_read_only_projection(self):
        source = ROOT / 'tconstruct/mixins/tconstruct'
        chests = (source / 'source-tinker-chests.rs').read_text()
        self.assertEqual(chests.count('tconstruct_inventory_api::project_slots('), 2)
        self.assertIn('(visible<CHEST_COLUMNS*CHEST_ROWS).then(|| first+visible)', chests)
        self.assertIn('(visible<48).then(|| first+visible)', chests)
        page = (source / 'source-smeltery-page.rs').read_text()
        self.assertIn('layout::melting_index(slots.len(), offset, menu_slot)', page)
        self.assertIn('view[..2].clone_from_slice(containers)', page)
        self.assertIn('project_with_containers(slots, offset, containers) != expected', page)
        self.assertIn('item != &containers[menu_slot]', page)
        for wrapper in ('game', 'lib'):
            self.assertIn('projection::project_slots', (source / f'chest-inventory-api-{wrapper}.rs').read_text())

    def test_generic_api_owns_no_machine_schema_or_host(self):
        source = (ROOT / 'pluto-lib/mixins/pluto_native_creative/inventory-api/src/projection.rs').read_text()
        self.assertIn('pub fn project_slots<T: Clone>', source)
        for name in ('horizon_', 'tconstruct', 'ItemStack', 'MENU_SIZE', 'CHEST_COLUMNS'):
            self.assertNotIn(name, source)

    def test_fixture_uses_real_crate_and_official_lock_guard(self):
        runner = (ROOT / 'tconstruct/tests/run_inventory_projection.py').read_text()
        self.assertIn('add_inventory_api(work / "Cargo.toml", root)', runner)
        self.assertIn('lock_fixture_to_host(work / "Cargo.toml", host, root)', runner)
        self.assertIn('"--locked", "--offline"', runner)

    def test_smeltery_no_opaque_policy_uses_one_recursive_native_query(self):
        source = ROOT / 'tconstruct/mixins/tconstruct'
        page = (source / 'source-smeltery-page.rs').read_text()
        self.assertIn('!super::tconstruct_inventory_api::contains_retained_wire(stack)', page)
        self.assertIn('held.count <= 1 || super::tconstruct_inventory_api::contains_retained_wire(held)', page)
        self.assertNotIn('data.wire.patch()', page)
        self.assertIn('no_retained_wire(page) && no_retained_wire(expected) && page == expected', page)
        self.assertIn('tconstruct_inventory_api::decode_item_canonical(&encoded)?', page)
        for wrapper in ('game', 'lib'):
            self.assertIn('pluto_stack_storage::{contains_retained_wire, decode_item_canonical}',
                          (source / f'chest-inventory-api-{wrapper}.rs').read_text())


if __name__ == '__main__':
    unittest.main()

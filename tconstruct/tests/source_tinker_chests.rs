pub use pluto_inventory_api as pluto_inventory;
#[path = "../mixins/tconstruct/chest-inventory-api-lib.rs"]
mod tconstruct_inventory_api;
#[path = "../mixins/tconstruct/source-tinker-chests.rs"]
mod rules;

#[cfg(test)]
mod projection_regressions {
    use super::rules;
    use horizon_items::{item_data::Wire, items::ItemStack};
    use horizon_world::nbt::Tag;
    use std::sync::Arc;

    fn reference(slots: &[Option<ItemStack>], offset: usize, columns: usize, visible: usize) -> Vec<Option<ItemStack>> {
        let mut output = vec![None; 54];
        for (index, target) in output.iter_mut().take(visible).enumerate() {
            if let Some(item) = slots.get(offset * columns + index) { *target = item.clone(); }
        }
        output
    }

    #[test]
    fn both_chest_views_match_original_mapping_with_opaque_items() {
        let mut item = ItemStack::new("minecraft:bundle", 3);
        item.damage = 7;
        item.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("future", Tag::Int(42))])));
        item.data_mut().bundle_contents.push(ItemStack::new("minecraft:brick", 1));
        item.data_mut().bundle_selected.0 = 0;
        let decoded = item.clone();
        item.data_mut().wire = Wire::new(decoded, Arc::<[u8]>::from([1, 2, 3]));
        item.damage += 1; // A display projection must preserve stale backing too.
        for count in [0, 1, 31, 32, 33, 64, 256] {
            let slots: Vec<_> = (0..count).map(|index| (index % 3 != 1).then(|| item.clone())).collect();
            for offset in [0, 1, 4, 28, 35, 99] {
                for (actual, expected) in [(rules::project(&slots, offset), reference(&slots, offset, 8, 32)),
                    (rules::project_side(&slots, offset), reference(&slots, offset, 6, 48))] {
                    assert!(crate::pluto_stack_storage::inventory_snapshot_matches(&expected, Some(&actual)));
                }
            }
        }
    }

    #[test]
    fn projected_stack_edits_do_not_change_source_cells() {
        let source = vec![Some(ItemStack::new("minecraft:stone", 3))];
        let mut page = rules::project(&source, 0);
        page[0].as_mut().unwrap().count = 1;
        assert_eq!(source[0].as_ref().unwrap().count, 3);
        assert!(page[32..].iter().all(Option::is_none));
    }
}

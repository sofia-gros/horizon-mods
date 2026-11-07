// Compile the actual transaction boundary helpers with official host types.
extern crate self as horizon_client;
pub use pluto_inventory_api as pluto_inventory;
#[path = "__STACK_STORAGE__"] pub mod pluto_stack_storage;
#[path = "__NBT_STORAGE__"] pub mod pluto_nbt_storage;

// __PRODUCTION_GUARDS__

#[cfg(test)]
mod host_tests {
    use super::*;
    use horizon_items::{item_data::Wire, item_nbt::item_to_nbt, items::ItemStack};
    use horizon_world::nbt::Tag;
    use std::sync::Arc;

    #[test]
    fn official_stack_roundtrip_preserves_count_damage_and_components() {
        let mut stack = ItemStack::new("minecraft:iron_pickaxe", 2);
        stack.damage = 7;
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("future", Tag::Int(19)), ("name", Tag::String("test:😀".into())),
        ])));
        let before = item_to_nbt(&stack);
        let encoded = encode_moving_item(&stack).unwrap();
        assert_eq!(encoded, before);
        assert_eq!(item_to_nbt(&stack), before);
    }

    #[test]
    fn retained_wire_is_rejected_even_when_public_patch_is_stale() {
        for stale in [false, true] {
            let mut stack = ItemStack::new("minecraft:iron_pickaxe", 1);
            stack.data_mut().wire = Wire::new(stack.clone(), Arc::<[u8]>::from([1, 2, 3]));
            if stale { stack.damage = 1; }
            assert_eq!(stack.wire_patch().is_none(), stale);
            let before = stack.clone();
            assert!(encode_moving_item(&stack).is_none());
            assert_eq!(stack, before);
            assert_eq!(stack.data().unwrap().wire.patch(), before.data().unwrap().wire.patch());
        }
        assert!(encode_moving_item(&ItemStack::new("minecraft:stone", 0)).is_none());
    }

    #[test]
    fn production_snapshot_guard_rejects_count_component_and_raw_storage_changes() {
        let before = vec![Some(ItemStack::new("minecraft:stone", 2)), None];
        let saved = Tag::compound([("future", Tag::Int(1))]);
        assert!(chute_snapshots_match(&before, &before, &saved, Some(&saved)));
        assert!(!chute_snapshots_match(&before, &before, &saved, None));
        let changed_raw = Tag::compound([("future", Tag::Int(2))]);
        assert!(!chute_snapshots_match(&before, &before, &saved, Some(&changed_raw)));
        let mut count = before.clone(); count[0].as_mut().unwrap().count += 1;
        assert!(!chute_snapshots_match(&before, &count, &saved, Some(&saved)));
        let mut component = before.clone();
        component[0].as_mut().unwrap().data_mut().set_other("minecraft:custom_data", Some(Tag::Int(3)));
        assert!(!chute_snapshots_match(&before, &component, &saved, Some(&saved)));
    }
    #[test]
    fn production_snapshot_guard_observes_retained_wire_changes() {
        let mut source = ItemStack::new("minecraft:iron_pickaxe", 1);
        source.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("kept", Tag::Int(7))])));
        let before = vec![Some(source), None];
        let saved = Tag::compound([("future", Tag::Int(1))]);
        for stale in [false, true] {
            let mut changed = before.clone();
            let item = changed[0].as_mut().unwrap();
            let mut decoded = item.clone();
            if stale { decoded.damage = 1; }
            item.data_mut().wire = Wire::new(decoded, Arc::<[u8]>::from([1, 2, 3]));
            assert_eq!(item.wire_patch().is_none(), stale);
            // Native identity intentionally ignores this opaque backing.
            assert!(item.same_item_same_components(before[0].as_ref().unwrap()));
            assert!(!chute_snapshots_match(&before, &changed, &saved, Some(&saved)));
            assert!(chute_snapshots_match(&changed, &changed, &saved, Some(&saved)));
            let mut other_visibility = changed.clone();
            let other = other_visibility[0].as_mut().unwrap();
            let mut decoded = other.clone();
            decoded.damage = if stale { 0 } else { 1 };
            other.data_mut().wire = Wire::new(decoded, Arc::<[u8]>::from([1, 2, 3]));
            assert_eq!(other.wire_patch().is_none(), !stale);
            assert!(!chute_snapshots_match(&changed, &other_visibility, &saved, Some(&saved)));
            let mut other_patch = changed.clone();
            let other = other_patch[0].as_mut().unwrap();
            other.data_mut().wire = Wire::new(other.clone(), Arc::<[u8]>::from([4, 5, 6]));
            assert!(!chute_snapshots_match(&changed, &other_patch, &saved, Some(&saved)));
        }
        assert!(!pluto_stack_storage::inventory_snapshot_matches(&[], None));
        assert!(pluto_stack_storage::inventory_snapshot_matches(&[], Some(&[])));
    }

    fn nested(field: usize, child: ItemStack) -> ItemStack {
        let mut parent = ItemStack::new("minecraft:chest", 1);
        match field {
            0 => parent.data_mut().container.push((7, child)),
            1 => parent.data_mut().charged.push(child),
            2 => parent.data_mut().pot_decorations = Some(Box::new([None, Some(child), None, None])),
            3 => parent.data_mut().bundle_contents.push(child),
            _ => unreachable!(),
        }
        parent
    }

    fn with_wire(mut item: ItemStack, stale: bool, bytes: [u8; 3]) -> ItemStack {
        let mut decoded = item.clone();
        if stale { decoded.damage = item.damage + 1; }
        item.data_mut().wire = Wire::new(decoded, Arc::<[u8]>::from(bytes));
        assert_eq!(item.wire_patch().is_none(), stale);
        item
    }

    #[test]
    fn nested_retained_wire_rejected_in_every_native_stack_field() {
        for field in 0..4 {
            for stale in [false, true] {
                let mut child = ItemStack::new("minecraft:brick", 1);
                child.data_mut().set_other("minecraft:custom_data", Some(Tag::Int(9)));
                let stack = nested(field, with_wire(child, stale, [1, 2, 3]));
                assert!(stack.data().unwrap().wire.patch().is_none());
                assert!(pluto_stack_storage::contains_retained_wire(&stack), "field {field}");
                assert!(encode_moving_item(&stack).is_none(), "field {field}, stale {stale}");
                // An unchanged opaque snapshot remains equal, but is not saveable.
                assert!(pluto_stack_storage::same_item_snapshot(&stack, &stack.clone()));
            }
        }
    }

    #[test]
    fn nested_wire_bytes_presence_and_usability_are_snapshot_identity() {
        for field in 0..4 {
            let mut child = ItemStack::new("minecraft:brick", 1);
            child.data_mut().set_other("minecraft:custom_data", Some(Tag::Int(9)));
            let plain = nested(field, child.clone());
            let usable = nested(field, with_wire(child.clone(), false, [1, 2, 3]));
            let stale = nested(field, with_wire(child.clone(), true, [1, 2, 3]));
            let other_bytes = nested(field, with_wire(child, false, [4, 5, 6]));
            for (before, after) in [(&plain, &usable), (&usable, &plain),
                                   (&usable, &stale), (&stale, &usable), (&usable, &other_bytes)] {
                assert!(before.same_item_same_components(after), "native equality omits nested wire");
                assert!(!pluto_stack_storage::same_item_snapshot(before, after), "field {field}");
            }
        }
    }

    #[test]
    fn nested_wire_callback_change_in_unselected_slot_aborts_transfer() {
        let saved = Tag::compound([("future", Tag::Int(1))]);
        for field in 0..4 {
            let mut child = ItemStack::new("minecraft:brick", 1);
            child.data_mut().set_other("minecraft:custom_data", Some(Tag::Int(9)));
            let moving = Some(ItemStack::new("minecraft:stone", 2));
            let before = vec![moving.clone(), Some(nested(field, child.clone()))];
            let changed = vec![moving, Some(nested(field, with_wire(child, true, [1, 2, 3])))];
            assert_eq!(before, changed, "native vector equality omits nested wire");
            assert!(!chute_snapshots_match(&before, &changed, &saved, Some(&saved)));
        }
    }

    #[test]
    fn ordinary_nested_stacks_still_roundtrip_and_structure_changes_reject() {
        for field in 0..4 {
            let mut child = ItemStack::new("minecraft:brick", 1);
            child.data_mut().set_other("minecraft:custom_data", Some(Tag::Int(9)));
            let stack = nested(field, child.clone());
            assert!(!pluto_stack_storage::contains_retained_wire(&stack));
            assert!(encode_moving_item(&stack).is_some(), "field {field}");
            assert!(pluto_stack_storage::same_item_snapshot(&stack, &stack.clone()));
            child.count = 100;
            // Charged projectiles use ItemStack.CODEC (1..=99); the other
            // native nested fields use ItemStackTemplate and preserve 100.
            assert_eq!(encode_moving_item(&nested(field, child.clone())).is_some(), field != 1);
            child.count = 0;
            let invalid = nested(field, child);
            assert!(encode_moving_item(&invalid).is_none(), "field {field}");
            assert!(!pluto_stack_storage::same_item_snapshot(&stack, &invalid));
        }
        let before = nested(0, ItemStack::new("minecraft:brick", 1));
        let mut moved = before.clone();
        moved.data_mut().container[0].0 = 8;
        assert!(!pluto_stack_storage::same_item_snapshot(&before, &moved));
    }

    #[test]
    fn wire_checks_reach_nested_mixed_stack_trees() {
        let mut child = ItemStack::new("minecraft:brick", 1);
        child.data_mut().set_other("minecraft:custom_data", Some(Tag::Int(9)));
        let mut before = child.clone();
        let mut after = with_wire(child, true, [1, 2, 3]);
        for depth in 0..32 {
            before = nested(depth % 4, before);
            after = nested(depth % 4, after);
        }
        assert!(!pluto_stack_storage::contains_retained_wire(&before));
        assert!(pluto_stack_storage::contains_retained_wire(&after));
        assert!(before.same_item_same_components(&after));
        assert!(!pluto_stack_storage::same_item_snapshot(&before, &after));
        assert!(encode_moving_item(&after).is_none());
    }

    #[test]
    fn bundle_selection_is_snapshot_state_but_not_saved_item_identity() {
        let mut child = ItemStack::new("minecraft:bundle", 1);
        child.data_mut().bundle_contents.push(ItemStack::new("minecraft:brick", 1));
        let mut changed = child.clone();
        changed.data_mut().bundle_selected.0 = 0;
        assert!(child.same_item_same_components(&changed));
        assert!(!pluto_stack_storage::same_item_snapshot(&child, &changed));
        assert_eq!(encode_moving_item(&child), encode_moving_item(&changed));
        assert!(encode_moving_item(&changed).is_some());
        for field in 0..4 {
            let before = nested(field, child.clone());
            let after = nested(field, changed.clone());
            assert!(before.same_item_same_components(&after));
            assert!(!pluto_stack_storage::same_item_snapshot(&before, &after));
        }
    }

}

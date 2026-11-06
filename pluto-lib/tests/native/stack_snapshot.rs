#[cfg(test)]
mod tests {
    use crate::{
        pluto_stack_snapshot::same_item_with_retained_baselines as exact,
        pluto_stack_storage::{encode_item_lossless, same_item_snapshot as observable},
    };
    use horizon_items::{
        item_data::{Wire, pluto_wire_read},
        items::ItemStack,
    };
    use std::sync::Arc;

    fn plain() -> ItemStack {
        ItemStack::new("minecraft:brick", 1)
    }
    fn retained(baseline: ItemStack, patch: &[u8]) -> ItemStack {
        let mut item = plain();
        item.data_mut().wire = Wire::new(baseline, Arc::<[u8]>::from(patch));
        item
    }
    fn stale_baseline(damage: u32) -> ItemStack {
        let mut item = plain();
        item.damage = damage;
        item
    }
    fn nested(field: usize, child: ItemStack) -> ItemStack {
        let mut parent = ItemStack::new("minecraft:chest", 1);
        match field {
            0 => parent.data_mut().container.push((3, child)),
            1 => parent.data_mut().charged.push(child),
            2 => {
                parent.data_mut().pot_decorations = Some(Box::new([None, Some(child), None, None]))
            }
            3 => parent.data_mut().bundle_contents.push(child),
            _ => unreachable!(),
        }
        parent
    }

    #[test]
    fn native_reader_borrows_the_actual_baseline_and_patch() {
        assert!(pluto_wire_read::retained(&Wire::default()).is_none());
        let mut baseline = stale_baseline(9);
        baseline.count = 7;
        let wire = Wire::new(baseline.clone(), Arc::<[u8]>::from([1, 2, 3]));
        let (decoded, patch) = pluto_wire_read::retained(&wire).unwrap();
        assert!(observable(decoded, &baseline));
        assert_eq!(
            (decoded.count, decoded.damage, patch),
            (7, 9, &[1, 2, 3][..])
        );
    }

    #[test]
    fn different_stale_baselines_with_identical_public_wire_facts_reject() {
        let before = retained(stale_baseline(1), &[4, 5, 6]);
        let after = retained(stale_baseline(2), &[4, 5, 6]);
        assert!(before.wire_patch().is_none());
        assert!(after.wire_patch().is_none());
        assert!(
            observable(&before, &after),
            "old API intentionally sees only public facts"
        );
        assert!(!exact(&before, &after));
        assert!(!exact(&after, &before));
    }

    #[test]
    fn independent_allocations_compare_by_values_not_addresses() {
        let before = retained(stale_baseline(9), &[1, 2, 3]);
        let equal = retained(stale_baseline(9), &[1, 2, 3]);
        assert!(exact(&before, &equal));
        assert!(exact(&before, &before.clone()));
        let changed = retained(stale_baseline(9), &[3, 2, 1]);
        assert!(!exact(&before, &changed));
        assert!(!exact(&before, &plain()));
    }

    #[test]
    fn baseline_count_component_and_bundle_selection_changes_reject() {
        let base = stale_baseline(9);
        let before = retained(base.clone(), &[1, 2]);
        let mut count = base.clone();
        count.count += 1;
        let mut component = base.clone();
        component.data_mut().set_other(
            "minecraft:custom_data",
            Some(horizon_world::nbt::Tag::Int(7)),
        );
        let mut selected = base.clone();
        selected.data_mut().bundle_selected.0 = 3;
        for baseline in [count, component, selected] {
            let after = retained(baseline, &[1, 2]);
            assert!(observable(&before, &after));
            assert!(!exact(&before, &after));
        }
    }

    #[test]
    fn every_ordinary_nested_stack_field_reaches_private_baselines() {
        for field in 0..4 {
            let before = nested(field, retained(stale_baseline(1), &[7, 8]));
            let after = nested(field, retained(stale_baseline(2), &[7, 8]));
            assert!(observable(&before, &after));
            assert!(!exact(&before, &after), "field {field}");
            assert!(exact(&before, &before.clone()));
        }
    }

    #[test]
    fn retained_baselines_can_themselves_contain_retained_baselines() {
        let mut left = nested(0, retained(stale_baseline(1), &[8, 9]));
        left.damage = 7;
        let mut right = nested(0, retained(stale_baseline(2), &[8, 9]));
        right.damage = 7;
        let before = retained(left, &[1, 2]);
        let after = retained(right, &[1, 2]);
        assert!(observable(&before, &after));
        assert!(!exact(&before, &after));
    }

    fn shared_baseline_graph(depth: usize, leaf_damage: u32) -> ItemStack {
        let mut node = stale_baseline(leaf_damage);
        for _ in 0..depth {
            let shared = Wire::new(node, Arc::<[u8]>::from([1, 2, 3]));
            let mut parent = ItemStack::new("minecraft:chest", 1);
            for slot in 0..2 {
                let mut child = plain();
                child.data_mut().wire = shared.clone();
                parent.data_mut().container.push((slot, child));
            }
            node = parent;
        }
        node
    }

    #[test]
    fn repeated_baseline_graphs_finish_without_expanding_every_path() {
        // Each graph has only O(depth) retained nodes but 2^depth paths. The two
        // graphs allocate independently; pointer identity cannot prove equality.
        let before = shared_baseline_graph(24, 1);
        let equal = shared_baseline_graph(24, 1);
        let changed = shared_baseline_graph(24, 2);
        assert!(exact(&before, &equal));
        assert!(!exact(&before, &changed));
    }

    #[test]
    fn exact_read_comparison_does_not_relax_save_or_change_inputs() {
        for item in [
            retained(plain(), &[1, 2]),
            retained(stale_baseline(9), &[1, 2]),
            nested(3, retained(stale_baseline(9), &[1, 2])),
        ] {
            let before = item.clone();
            assert!(exact(&item, &before));
            assert!(encode_item_lossless(&item).is_none());
            assert!(exact(&item, &before));
        }
        assert!(exact(&plain(), &plain()));
        assert!(encode_item_lossless(&plain()).is_some());
    }
}

// Additional cross-service coverage using the unchanged packaged modules and
// actual host types. This does not call a Game or grant write authority.
#[cfg(test)]
mod tests {
    use crate::{items::ItemStack, pluto_component_view::ItemFacts,
        pluto_inventory::equipment::{EquipmentSlot, EquipmentSlots, Slot}};
    use crate::item_data::Wire;
    use std::sync::Arc;

    fn nested(field: usize, child: ItemStack) -> ItemStack {
        let mut parent = ItemStack::new("minecraft:chest", 3);
        parent.damage = 7;
        match field {
            0 => parent.data_mut().container.push((4, child)),
            1 => parent.data_mut().charged.push(child),
            2 => parent.data_mut().pot_decorations = Some(Box::new([None, Some(child), None, None])),
            3 => parent.data_mut().bundle_contents.push(child),
            _ => unreachable!(),
        }
        parent
    }

    #[test]
    fn nested_retained_wire_makes_component_facts_unavailable() {
        for field in 0..4 {
            let child = ItemStack::new("minecraft:brick", 1);
            let plain = nested(field, child.clone());
            assert!(ItemFacts::capture(&plain).view().components.is_some());
            for stale in [false, true] {
                let mut opaque_child = child.clone();
                let mut decoded = child.clone();
                if stale { decoded.damage += 1; }
                opaque_child.data_mut().wire = Wire::new(decoded, Arc::<[u8]>::from([4, 5, 6]));
                assert_eq!(opaque_child.wire_patch().is_none(), stale);
                let opaque = nested(field, opaque_child);
                assert!(opaque.data().unwrap().wire.patch().is_none(), "wire exists only below root");
                let before = opaque.clone();
                let facts = ItemFacts::capture(&opaque);
                let view = facts.view();
                assert_eq!((view.id, view.count, view.damage), ("minecraft:chest", 3, 7));
                assert!(view.components.is_none(), "field {field}, stale {stale}");
                let snapshot = crate::pluto_equipment::Snapshot::capture(
                    7, "test:world", &[None, None, None, None], Some(&opaque), None);
                let equipment = crate::pluto_protection_facts::Equipment::capture(&snapshot);
                let Slot::Item(captured) = equipment.slot(EquipmentSlot::MainHand) else {
                    panic!("real captured stack must remain available");
                };
                assert_eq!((captured.id, captured.count, captured.damage), (view.id, view.count, view.damage));
                assert!(captured.components.is_none());
                assert!(crate::pluto_stack_storage::same_item_snapshot(&opaque, &before));
            }
        }
    }
}

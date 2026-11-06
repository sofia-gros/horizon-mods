include!("pluto_storage_fixture.rs");
include!("pluto_potion_fixture.rs");
#[path = "../../mixins/tconstruct/chest-inventory-api-lib.rs"]
mod tconstruct_inventory_api;
#[path = "../../mixins/tconstruct/source-smeltery-layout.rs"]
mod tconstruct_source_smeltery_layout;
#[path = "../../mixins/tconstruct/source-smeltery-page.rs"]
mod tconstruct_source_smeltery_page;
#[path = "../../mixins/tconstruct/source-smeltery-state.rs"]
mod tconstruct_source_smeltery_state;
#[path = "../../mixins/tconstruct/source-smeltery-tank.rs"]
mod tconstruct_source_smeltery_tank;
#[path = "../../mixins/tconstruct/source-smeltery-fuel.rs"]
mod tconstruct_source_smeltery_fuel;
#[path = "../../mixins/tconstruct/source-melter-state.rs"]
mod tconstruct_source_melter_state;

#[cfg(test)]
mod canonical_tests {
    use super::*;
    use horizon_items::{item_data::Wire, item_nbt::{item_from_nbt, item_to_nbt}, items::ItemStack};
    use horizon_world::nbt::{JavaString, Tag};
    use std::sync::Arc;
    use tconstruct_source_smeltery_state::Smeltery;

    fn metadata_item(count: u32) -> ItemStack {
        let mut item = ItemStack::new("minecraft:iron_pickaxe", count);
        item.damage = 7;
        item.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound([
            (JavaString(vec![0xd800, 0x78]), Tag::String(JavaString(vec![0xdc00]))),
            ("float".into(), Tag::Float(0x7fc0_0123)),
            ("double".into(), Tag::Double(0x8000_0000_0000_0000)),
            ("nested".into(), Tag::List(vec![Tag::IntArray(vec![1, -3])])),
        ].into_iter().collect())));
        item
    }

    fn stack_cases() -> Vec<ItemStack> {
        let mut cases: Vec<_> = [0, 1, 2, 99, 100, 127, 128, u32::MAX]
            .into_iter().map(metadata_item).collect();
        for field in 0..4 {
            for count in [0, 1, 99, 100] {
                let child = metadata_item(count);
                let mut parent = ItemStack::new("minecraft:chest", 7);
                match field {
                    0 => parent.data_mut().container.push((3, child)),
                    1 => parent.data_mut().charged.push(child),
                    2 => parent.data_mut().pot_decorations = Some(Box::new([None, Some(child), None, None])),
                    3 => parent.data_mut().bundle_contents.push(child),
                    _ => unreachable!(),
                }
                cases.push(parent);
            }
        }
        for stale in [false, true] {
            let mut item = metadata_item(7);
            let mut decoded = item.clone();
            if stale { decoded.damage += 1; }
            item.data_mut().wire = Wire::new(decoded, Arc::<[u8]>::from([4, 5, 6]));
            cases.push(item);
        }
        cases
    }

    fn raw_cases() -> Vec<Tag> {
        let mut cases: Vec<_> = stack_cases().iter().map(item_to_nbt).collect();
        let canonical = item_to_nbt(&metadata_item(3));
        for (key, value) in [
            ("future", Tag::Int(42)), ("Count", Tag::Byte(3)),
            ("count", Tag::Int(0)), ("count", Tag::Int(100)),
            ("count", Tag::Long(3)), ("id", Tag::String(JavaString(vec![0xd800]))),
        ] {
            let mut raw = canonical.clone();
            let Tag::Compound(fields) = &mut raw else { unreachable!() };
            fields.insert(key.into(), value);
            cases.push(raw);
        }
        let mut missing_count = canonical.clone();
        let Tag::Compound(fields) = &mut missing_count else { unreachable!() };
        fields.remove(&"count".into());
        cases.extend([missing_count, Tag::End, Tag::Int(9), Tag::List(vec![]), Tag::compound(Vec::<(&str, Tag)>::new())]);
        cases
    }

    #[test]
    fn native_decode_matches_previous_tag_roundtrip_matrix() {
        for raw in raw_cases() {
            let before = raw.clone();
            let previous = item_from_nbt(&raw).filter(|decoded| item_to_nbt(decoded) == raw);
            let actual = pluto_stack_storage::decode_item_canonical(&raw);
            assert_eq!(actual.is_some(), previous.is_some(), "raw: {raw:?}");
            if let (Some(actual), Some(previous)) = (actual, previous) {
                assert!(pluto_stack_storage::same_item_snapshot(&actual, &previous));
                assert_eq!(item_to_nbt(&actual), raw);
            }
            assert_eq!(raw, before);
        }
        let canonical = item_to_nbt(&metadata_item(3));
        let decoded = pluto_stack_storage::decode_item_canonical(&canonical).unwrap();
        assert_eq!(item_to_nbt(&decoded), canonical, "UTF-16 and float bits stay native Tags");
        let mut opaque = canonical.clone();
        let Tag::Compound(fields) = &mut opaque else { unreachable!() };
        fields.insert("future".into(), Tag::Int(42));
        assert!(item_from_nbt(&opaque).is_some());
        assert!(pluto_stack_storage::decode_item_canonical(&opaque).is_none());
    }

    #[test]
    fn canonicality_is_not_saved_or_runtime_snapshot_identity() {
        let mut source = metadata_item(3);
        source.data_mut().bundle_selected.0 = 4;
        let decoded = pluto_stack_storage::decode_item_canonical(&item_to_nbt(&source)).unwrap();
        assert!(!pluto_stack_storage::same_item_snapshot(&source, &decoded));
        source.data_mut().wire = Wire::new(source.clone(), Arc::<[u8]>::from([1, 2, 3]));
        assert!(pluto_stack_storage::encode_item_lossless(&source).is_none());
        assert!(pluto_stack_storage::decode_item_canonical(&item_to_nbt(&source)).is_some());
    }

    // Frozen pre-extraction implementation from 1cbbe64f. Keeping the complete
    // bounded function makes the parity check include empty-slot and page policy.
    fn previous_single_insert(raw: &Tag, index: usize, held: &ItemStack) -> Option<(Tag, ItemStack, ItemStack)> {
        if held.count <= 1 || pluto_stack_storage::contains_retained_wire(held) { return None; }
        let mut one = held.clone(); one.count = 1;
        let encoded = item_to_nbt(&one);
        let decoded = item_from_nbt(&encoded)?;
        if decoded.count != 1 || !one.same_item_same_components(&decoded)
            || item_to_nbt(&decoded) != encoded { return None; }
        let Tag::Compound(mut fields) = raw.clone() else { return None; };
        let Some(Tag::List(slots)) = fields.get_mut(&"slots".into()) else { return None; };
        for raw_slot in slots.iter() {
            if matches!(raw_slot, Tag::Compound(map) if map.is_empty()) { continue; }
            let decoded = item_from_nbt(raw_slot)?;
            if item_to_nbt(&decoded) != *raw_slot { return None; }
        }
        let slot = slots.get_mut(index)?;
        if !matches!(slot, Tag::Compound(map) if map.is_empty()) { return None; }
        *slot = encoded;
        let Some(Tag::IntArray(progress)) = fields.get_mut(&"progress".into()) else { return None; };
        *progress.get_mut(index)? = 0;
        let mut remainder = held.clone(); remainder.count -= 1;
        Some((Tag::Compound(fields), one, remainder))
    }

    #[test]
    fn single_insert_matches_previous_whole_function_matrix() {
        let empty = Tag::compound(Vec::<(&str, Tag)>::new());
        for saved in raw_cases() {
            let raw = Tag::compound([("slots", Tag::List(vec![empty.clone(), saved])),
                ("progress", Tag::IntArray(vec![9, 42])), ("extension", Tag::Byte(3))]);
            for held in stack_cases() {
                let before = held.clone();
                for index in 0..3 {
                    let previous = previous_single_insert(&raw, index, &held);
                    let actual = tconstruct_source_smeltery_page::prepare_single_insert(&raw, index, &held);
                    assert_eq!(actual.is_some(), previous.is_some());
                    if let (Some((raw_a, one_a, rest_a)), Some((raw_b, one_b, rest_b))) = (actual, previous) {
                        assert_eq!(raw_a, raw_b);
                        assert!(pluto_stack_storage::same_item_snapshot(&one_a, &one_b));
                        assert!(pluto_stack_storage::same_item_snapshot(&rest_a, &rest_b));
                    }
                }
                assert!(pluto_stack_storage::same_item_snapshot(&held, &before));
            }
        }
    }

    // Frozen inventory-only method from 1cbbe64f, expressed as a free function.
    fn previous_inventory_patch(state: &Smeltery, raw: &Tag) -> Option<Tag> {
        let before = Smeltery::from_nbt(raw)?;
        if state.slots.len() != before.slots.len() || state.progress.len() != before.progress.len()
            || raw.get("Items").is_some() { return None; }
        let mut unchanged = state.clone(); unchanged.slots = before.slots.clone(); unchanged.progress = before.progress.clone();
        if unchanged.to_nbt() != before.to_nbt() { return None; }
        let Tag::List(raw_slots) = raw.get("slots")? else { return None; };
        let empty = || Tag::compound(Vec::<(&str, Tag)>::new());
        let mut slots = Vec::with_capacity(state.slots.len());
        for (index, slot) in state.slots.iter().enumerate() {
            let old = before.slots[index].as_ref().map(item_to_nbt).unwrap_or_else(empty);
            if raw_slots[index] != old { return None; }
            let next = if let Some(item) = slot {
                if item.count == 0 || item.wire_patch().is_some() || item.data().is_some_and(|data| data.wire.patch().is_some()) { return None; }
                let encoded = item_to_nbt(item); let decoded = item_from_nbt(&encoded)?;
                if decoded.count != item.count || decoded.damage != item.damage || !decoded.same_item_same_components(item)
                    || item_to_nbt(&decoded) != encoded { return None; }
                encoded
            } else { empty() };
            if state.progress[index] != before.progress[index] && state.progress[index] != 0 { return None; }
            slots.push(next);
        }
        let Tag::Compound(mut fields) = raw.clone() else { return None; };
        fields.insert("slots".into(), Tag::List(slots));
        fields.insert("progress".into(), Tag::IntArray(state.progress.clone()));
        Some(Tag::Compound(fields))
    }

    #[test]
    fn inventory_patch_matches_previous_whole_function_matrix() {
        let mut initial = Smeltery::new(); initial.resize(1, 1080);
        for saved in raw_cases() {
            let mut raw = initial.to_nbt();
            let Tag::Compound(fields) = &mut raw else { unreachable!() };
            fields.insert("slots".into(), Tag::List(vec![saved]));
            fields.insert("extension".into(), Tag::Int(42));
            for item in std::iter::once(None).chain(stack_cases().into_iter().map(Some)) {
                for progress in [0, 1, -1] {
                    let mut proposed = initial.clone();
                    proposed.slots[0] = item.clone(); proposed.progress[0] = progress;
                    let before = proposed.to_nbt();
                    assert_eq!(proposed.inventory_to_nbt_preserving(&raw), previous_inventory_patch(&proposed, &raw));
                    assert_eq!(proposed.to_nbt(), before);
                }
            }
        }
    }
    #[test]
    fn inventory_patch_rejects_nested_wire_but_keeps_existing_plain_policy() {
        let mut initial = Smeltery::new(); initial.resize(1, 1080);
        let raw = initial.to_nbt();
        for field in 0..4 {
            for stale in [false, true] {
                let child = metadata_item(1);
                let mut opaque_child = child.clone();
                let mut decoded = child.clone();
                if stale { decoded.damage += 1; }
                opaque_child.data_mut().wire = Wire::new(decoded, Arc::<[u8]>::from([7, 8, 9]));
                assert_eq!(opaque_child.wire_patch().is_none(), stale);
                let parent = |child: ItemStack| {
                    let mut item = ItemStack::new("minecraft:chest", 3);
                    match field {
                        0 => item.data_mut().container.push((4, child)),
                        1 => item.data_mut().charged.push(child),
                        2 => item.data_mut().pot_decorations = Some(Box::new([None, Some(child), None, None])),
                        3 => item.data_mut().bundle_contents.push(child),
                        _ => unreachable!(),
                    }
                    item
                };
                let mut plain = initial.clone(); plain.slots[0] = Some(parent(child));
                assert!(plain.inventory_to_nbt_preserving(&raw).is_some());
                assert_eq!(plain.inventory_to_nbt_preserving(&raw), previous_inventory_patch(&plain, &raw));
                let mut opaque = initial.clone(); opaque.slots[0] = Some(parent(opaque_child));
                let before = opaque.slots.clone();
                assert!(previous_inventory_patch(&opaque, &raw).is_some(), "old root-only gate passed nested wire");
                assert!(opaque.inventory_to_nbt_preserving(&raw).is_none(), "field {field}, stale {stale}");
                assert!(pluto_stack_storage::inventory_snapshot_matches(&before, Some(&opaque.slots)));
            }
        }
        assert_eq!(initial.inventory_to_nbt_preserving(&raw), Some(raw.clone()));
        let mut alias = raw.clone();
        let Tag::Compound(fields) = &mut alias else { unreachable!() };
        fields.insert("Items".into(), Tag::List(vec![]));
        assert!(initial.inventory_to_nbt_preserving(&alias).is_none());
        let mut progress = initial.clone(); progress.progress[0] = 1;
        assert!(progress.inventory_to_nbt_preserving(&raw).is_none());
        assert_eq!(raw, initial.to_nbt());
    }

    fn previous_tank_patch(state: &Smeltery, raw: &Tag) -> Option<Tag> {
        let before = Smeltery::from_nbt(raw)?;
        let mut unchanged = state.clone(); unchanged.tank = before.tank.clone();
        if unchanged.to_nbt() != before.to_nbt() { return None; }
        for alias in ["Tank", "Fluids"] { if raw.get(alias).is_some() { return None; } }
        let tank = state.tank.to_nbt_preserving(raw.get("tank")?)?;
        let Tag::Compound(mut fields) = raw.clone() else { return None; };
        fields.insert("tank".into(), tank);
        Some(Tag::Compound(fields))
    }

    #[test]
    fn owned_compound_patches_match_previous_writes_and_preserve_raw_extensions() {
        let mut initial = Smeltery::new(); initial.resize(1, 1080);
        initial.slots[0] = Some(metadata_item(2)); initial.progress[0] = 7;
        let mut raw = initial.to_nbt();
        let key = JavaString(vec![0xd800, 0x78]);
        let value = Tag::List(vec![Tag::String(JavaString(vec![0xdc00])),
            Tag::Float(0x7fc0_0123), Tag::Double(0x8000_0000_0000_0000)]);
        let Tag::Compound(fields) = &mut raw else { unreachable!() };
        fields.insert(key.clone(), value.clone());
        let mut cases = vec![raw.clone(), Tag::Int(3)];
        for alias in ["Tank", "Fluids", "Items"] {
            let mut changed = raw.clone();
            let Tag::Compound(fields) = &mut changed else { unreachable!() };
            fields.insert(alias.into(), Tag::List(vec![])); cases.push(changed);
        }
        let mut opaque_slot = raw.clone();
        let Some(Tag::List(slots)) = opaque_slot.get_mut("slots") else { unreachable!() };
        let Tag::Compound(fields) = &mut slots[0] else { unreachable!() };
        fields.insert("future_item_field".into(), Tag::Int(42)); cases.push(opaque_slot.clone());
        let mut missing_progress = raw.clone();
        let Tag::Compound(fields) = &mut missing_progress else { unreachable!() };
        fields.remove(&"progress".into()); cases.push(missing_progress);
        let mut tank = initial.clone(); assert!(tank.tank.insert("minecraft:water", 20));
        let mut inventory = initial.clone(); inventory.slots[0] = None; inventory.progress[0] = 0;
        for original in cases {
            let before = original.clone();
            assert_eq!(tank.tank_to_nbt_preserving(&original), previous_tank_patch(&tank, &original));
            assert_eq!(inventory.inventory_to_nbt_preserving(&original), previous_inventory_patch(&inventory, &original));
            assert_eq!(original, before);
        }
        let saved = tank.tank_to_nbt_preserving(&opaque_slot).unwrap();
        assert_eq!(saved.get("slots"), opaque_slot.get("slots"), "tank-only proposal preserves opaque raw items");
        assert!(inventory.inventory_to_nbt_preserving(&opaque_slot).is_none());
        for saved in [tank.tank_to_nbt_preserving(&raw).unwrap(), inventory.inventory_to_nbt_preserving(&raw).unwrap()] {
            let Tag::Compound(fields) = saved else { unreachable!() };
            assert_eq!(fields.get(&key), Some(&value));
        }
        let mut unrelated = tank.clone(); unrelated.fuel += 1;
        assert!(unrelated.tank_to_nbt_preserving(&raw).is_none());
        let mut bad_progress = inventory.clone(); bad_progress.progress[0] = 9;
        assert!(bad_progress.inventory_to_nbt_preserving(&raw).is_none());
    }

}

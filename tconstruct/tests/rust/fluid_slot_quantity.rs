use super::*;
use horizon_world::nbt::Tag;
use std::cell::RefCell;

fn sample(kind: usize, count: u32) -> ItemStack {
    let ids = ["minecraft:bucket", "minecraft:water_bucket", "minecraft:bowl", "minecraft:glass_bottle",
        "minecraft:honey_bottle", "tconstruct:copper_can", "tconstruct:seared_lantern",
        "minecraft:potion", "minecraft:splash_potion", "tconstruct:pickaxe", "example:unknown",
        "minecraft:water_bucket", "minecraft:bucket", "tconstruct:copper_can", "tconstruct:seared_lantern"];
    let mut item = ItemStack::new(ids[kind], count);
    match kind {
        7 | 8 => {
            item = item.with_potion("minecraft:water");
            if kind == 8 { item.data_mut().potion_extra.push(("custom_color".into(), Tag::Int(0x123456))); }
        }
        9 => { item.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_tank_capacity", Tag::Int(144))]))); }
        11 => item.damage = 1,
        12 => { item.data_mut(); } // Some(default) remains different from None.
        13 => { item.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("future", Tag::Int(99))]))); }
        14 => { item.data_mut().set_other("minecraft:custom_data", Some(Tag::Int(7))); }
        _ => {}
    }
    item
}
fn debug(value: &impl std::fmt::Debug) -> String { format!("{value:?}") }
fn destinations(incoming: &ItemStack) -> Vec<Option<ItemStack>> {
    let mut all = vec![None];
    for count in [0, 1, 16, 64, u32::MAX] {
        let mut same = incoming.clone(); same.count = count; all.push(Some(same));
    }
    let mut damage = incoming.clone(); damage.damage = damage.damage.wrapping_add(1); all.push(Some(damage));
    let mut extra = incoming.clone(); extra.data_mut().set_other("example:extension", Some(Tag::Int(4))); all.push(Some(extra));
    all.push(Some(ItemStack::new("minecraft:diamond", 1)));
    all
}

#[test]
fn clicks_match_frozen_body_values_and_known_fluid_trace() {
    let mut cases = 0;
    for kind in 0..15 {
        for count in [0, 1, 2, 16, 65, u32::MAX] {
            let incoming = sample(kind, count);
            for cell in destinations(&incoming) {
                for index in [0, 1, 2] {
                    for right in [false, true] {
                        for allow_color in [false, true] {
                            for known in [false, true] {
                                let mut old = [cell.clone(), cell.clone()]; let mut new = old.clone();
                                let mut old_carried = Some(incoming.clone()); let mut new_carried = old_carried.clone();
                                let old_trace = RefCell::new(Vec::new()); let new_trace = RefCell::new(Vec::new());
                                let expected = quantity_legacy::click_with_color(&mut old, &mut old_carried, index, right,
                                    &|id| { old_trace.borrow_mut().push(id.to_owned()); known }, allow_color);
                                let actual = click_with_color(&mut new, &mut new_carried, index, right,
                                    &|id| { new_trace.borrow_mut().push(id.to_owned()); known }, allow_color);
                                assert_eq!(actual, expected);
                                assert_eq!(debug(&(&new, &new_carried)), debug(&(&old, &old_carried)));
                                assert_eq!(new_trace.into_inner(), old_trace.into_inner()); cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 19440);
}

#[test]
fn quick_input_matches_frozen_body_values_and_known_fluid_trace() {
    let mut cases = 0;
    for kind in 0..15 {
        for count in [0, 1, 2, 16, 65, u32::MAX] {
            let incoming = sample(kind, count);
            for cell in destinations(&incoming) {
                for allow_color in [false, true] {
                    for known in [false, true] {
                        let mut old = [cell.clone(), Some(ItemStack::new("example:output", 3))]; let mut new = old.clone();
                        let mut old_carried = Some(incoming.clone()); let mut new_carried = old_carried.clone();
                        let old_trace = RefCell::new(Vec::new()); let new_trace = RefCell::new(Vec::new());
                        let expected = quantity_legacy::quick_input_with_color(&mut old, &mut old_carried,
                            &|id| { old_trace.borrow_mut().push(id.to_owned()); known }, allow_color);
                        let actual = quick_input_with_color(&mut new, &mut new_carried,
                            &|id| { new_trace.borrow_mut().push(id.to_owned()); known }, allow_color);
                        assert_eq!(actual, expected);
                        assert_eq!(debug(&(&new, &new_carried)), debug(&(&old, &old_carried)));
                        assert_eq!(new_trace.into_inner(), old_trace.into_inner()); cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 3240);
}

#[test]
fn empty_cursor_extraction_and_missing_input_remain_legacy_operations() {
    let mut cases = 0;
    for kind in 0..15 {
        for count in [0, 1, 2, 16, 65, u32::MAX] {
            for index in [0, 1, 2] {
                for right in [false, true] {
                    let mut old = [Some(sample(kind, count)), Some(sample(kind, count))]; let mut new = old.clone();
                    let mut old_carried = None; let mut new_carried = None;
                    let forbidden = |_: &str| -> bool { panic!("extraction must not query fluid admission") };
                    let expected = quantity_legacy::click_with_color(&mut old, &mut old_carried, index, right, &forbidden, true);
                    let actual = click_with_color(&mut new, &mut new_carried, index, right, &forbidden, true);
                    assert_eq!(actual, expected); assert_eq!(debug(&(&new, &new_carried)), debug(&(&old, &old_carried)));cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 540);
    let mut slots = [None, None]; let mut held = None;
    assert!(!quick_input_with_color(&mut slots, &mut held, &|_| panic!("missing input"), true));
    assert!(!click_with_color(&mut slots, &mut held, 0, false, &|_| panic!("empty input"), true));
}

#[test]
fn every_admission_family_rejects_zero_incoming_before_fluid_queries() {
    for kind in 0..15 {
        for allow_color in [false, true] {
            assert!(!accepts_with_color(&sample(kind, 0), &|_| panic!("zero input must not reach fluid query"), allow_color));
        }
    }
    // Exercise all distinct successful parsing families with actual helpers.
    for kind in [0, 4, 5, 6, 7, 8, 9, 13] {
        assert!(accepts_with_color(&sample(kind, 1), &|_| true, true), "kind={kind}");
    }
}

#[test]
fn exact_legacy_component_policy_keeps_opaque_receiving_value() {
    use horizon_items::item_data::Wire;
    use std::sync::Arc;
    let mut receiving = ItemStack::new("minecraft:potion", 0).with_potion("minecraft:water");
    let baseline = receiving.clone(); receiving.data_mut().wire = Wire::new(baseline, Arc::from([1u8]));
    let mut incoming = ItemStack::new("minecraft:potion", 1).with_potion("minecraft:water");
    let baseline = incoming.clone(); incoming.data_mut().wire = Wire::new(baseline, Arc::from([2u8]));
    assert!(accepts_with_color(&incoming, &|_| true, false));
    for quick in [false, true] {
        let mut old = [Some(receiving.clone()), None]; let mut new = old.clone();
        let mut old_carried = Some(incoming.clone()); let mut new_carried = old_carried.clone();
        let (expected, actual) = if quick {
            (quantity_legacy::quick_input_with_color(&mut old, &mut old_carried, &|_| true, false),
             quick_input_with_color(&mut new, &mut new_carried, &|_| true, false))
        } else {
            (quantity_legacy::click_with_color(&mut old, &mut old_carried, 0, false, &|_| true, false),
             click_with_color(&mut new, &mut new_carried, 0, false, &|_| true, false))
        };
        assert_eq!((actual, expected), (true, true));
        assert_eq!(debug(&(&new, &new_carried)), debug(&(&old, &old_carried)));
        let mut merged = receiving.clone(); merged.count = 1;
        assert_eq!(debug(&new[0]), debug(&Some(merged))); assert!(new_carried.is_none());
    }
}

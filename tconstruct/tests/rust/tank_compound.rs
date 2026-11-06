use super::*;
use horizon_items::{item_data::Wire, items::ItemStack};
use horizon_world::nbt::{Compound, JavaString, Tag};
use std::sync::Arc;
use tconstruct_source_smeltery_tank::{FluidAmount, SmelteryTank};
use tconstruct_source_casting_tank::CastingTank;

fn set(raw: &mut Tag, key: &str, value: Tag) {
    let Tag::Compound(fields) = raw else { panic!("test compound"); };
    fields.insert(key.into(), value);
}
fn opaque(raw: &mut Tag) {
    let Tag::Compound(fields) = raw else { panic!("test compound"); };
    fields.insert(JavaString(vec![0xd800, 0x78]), Tag::List(vec![
        Tag::String(JavaString(vec![0xdc00])), Tag::Float(0x7fc0_1234),
        Tag::Double(0x8000_0000_0000_0000), Tag::IntArray(vec![3, -8]),
    ]));
}
fn smeltery() -> SmelteryTank {
    let mut tank = SmelteryTank::new(4000);
    assert!(tank.insert("example:copper", 100));
    let tag = Tag::compound([("variant", Tag::Int(2))]);
    assert!(tank.insert_tagged("example:water", Some(&tag), 200));
    tank
}
fn casting() -> CastingTank {
    let mut tank = CastingTank::new();
    assert!(tank.tank.insert("minecraft:water", 1000));
    tank.slots[0] = Some(ItemStack::new("minecraft:bucket", 2));
    tank
}
fn compare_smeltery(tank: &SmelteryTank, raw: &Tag) {
    let old_raw = raw.clone(); let old_state = tank.clone();
    assert_eq!(tank.to_nbt_preserving(raw), tank.legacy_to_nbt_preserving(raw),
        "state={tank:?}, raw={raw:?}");
    assert_eq!(*raw, old_raw); assert_eq!(*tank, old_state);
}
fn compare_casting(tank: &CastingTank, raw: &Tag) {
    let old_raw = raw.clone(); let old_state = format!("{tank:?}");
    assert_eq!(tank.to_nbt_preserving(raw), tank.legacy_to_nbt_preserving(raw),
        "state={tank:?}, raw={raw:?}");
    assert_eq!(*raw, old_raw); assert_eq!(format!("{tank:?}"), old_state);
}

#[test]
fn smeltery_complete_legacy_method_matches_owned_patch_matrix() {
    let base = smeltery(); let canonical = base.to_nbt();
    let mut raws = vec![canonical.clone(), Tag::End, Tag::Int(7), Tag::List(vec![]), Tag::compound([] as [(&str, Tag); 0])];
    for field in ["Capacity", "Fluids", "Fluid", "Tank", "FluidName", "Amount", "Tag"] {
        let mut raw = canonical.clone(); set(&mut raw, field, Tag::Int(8)); raws.push(raw);
    }
    for field in ["FluidName", "Amount", "Tag", "fluid"] {
        let mut raw = canonical.clone(); let Tag::Compound(map) = &mut raw else { unreachable!() };
        let Some(Tag::List(rows)) = map.get_mut(&"fluids".into()) else { unreachable!() };
        set(&mut rows[0], field, Tag::Int(8)); raws.push(raw);
    }
    for mode in 0..12 {
        let mut raw = canonical.clone();
        match mode {
            0 => opaque(&mut raw),
            1 => set(&mut raw, "capacity", Tag::Int(4000)),
            2 => set(&mut raw, "capacity", Tag::Long(-1)),
            3 => set(&mut raw, "fluids", Tag::List(vec![])),
            4 => set(&mut raw, "fluids", Tag::Int(1)),
            _ => {
                let Tag::Compound(map) = &mut raw else { unreachable!() };
                let Some(Tag::List(rows)) = map.get_mut(&"fluids".into()) else { unreachable!() };
                match mode {
                    5 => opaque(&mut rows[0]),
                    6 => set(&mut rows[0], "id", Tag::String(JavaString(vec![0xd800]))),
                    7 => set(&mut rows[0], "amount", Tag::Long(0)),
                    8 => set(&mut rows[0], "amount", Tag::Long(-1)),
                    9 => set(&mut rows[0], "fluid_tag", Tag::Int(1)),
                    10 => { let row = rows[0].clone(); rows.push(row); }
                    11 => rows[0] = Tag::Int(2),
                    _ => unreachable!(),
                }
            }
        }
        raws.push(raw);
    }
    let mut states = Vec::new();
    for mode in 0..11 {
        let mut next = base.clone();
        match mode {
            0 => {}, 1 => next.fluids.reverse(), 2 => next.fluids[0].amount = 9,
            3 => { next.fluids.remove(0); }, 4 => next.fluids.clear(),
            5 => next.fluids.push(FluidAmount { id: "new:fluid".into(), tag: None, amount: 5 }),
            6 => next.fluids[0].tag = Some(Tag::compound([("changed", Tag::Int(3))])),
            7 => next.capacity = 1, 8 => next.fluids[0].amount = 0,
            9 => next.fluids[0].id.clear(),
            10 => next.fluids.push(next.fluids[0].clone()), _ => unreachable!(),
        }
        states.push(next);
    }
    let mut cases = 0;
    for raw in &raws { for tank in &states { compare_smeltery(tank, raw); cases += 1; } }
    assert_eq!(cases, 308);
}

#[test]
fn casting_complete_legacy_method_matches_raw_slot_and_alias_matrix() {
    let base = casting(); let canonical = base.to_nbt();
    let mut raws = vec![canonical.clone(), Tag::End, Tag::Int(7), Tag::List(vec![]), Tag::compound([] as [(&str, Tag); 0])];
    for field in ["Items", "Tank", "Fluid", "lastRedstone"] {
        let mut raw = canonical.clone(); set(&mut raw, field, Tag::Int(8)); raws.push(raw);
    }
    for field in ["FluidName", "Amount", "Tag"] {
        let mut raw = canonical.clone(); let Tag::Compound(map) = &mut raw else { unreachable!() };
        set(map.get_mut(&"tank".into()).unwrap(), field, Tag::Int(8)); raws.push(raw);
    }
    for mode in 0..12 {
        let mut raw = canonical.clone();
        match mode {
            0 => opaque(&mut raw), 1 => set(&mut raw, "redstone", Tag::Byte(2)),
            2 => set(&mut raw, "redstone", Tag::Int(0)), 3 => set(&mut raw, "tank", Tag::Int(1)),
            4 => set(&mut raw, "slots", Tag::List(vec![])), 5 => set(&mut raw, "slots", Tag::Int(1)),
            6 => {
                let Tag::Compound(map) = &mut raw else { unreachable!() };
                opaque(map.get_mut(&"tank".into()).unwrap());
            }
            _ => {
                let Tag::Compound(map) = &mut raw else { unreachable!() };
                let Some(Tag::List(slots)) = map.get_mut(&"slots".into()) else { unreachable!() };
                match mode {
                    7 => opaque(&mut slots[0]), 8 => opaque(&mut slots[1]),
                    9 => set(&mut slots[0], "count", Tag::Int(0)),
                    10 => set(&mut slots[0], "Count", Tag::Byte(2)),
                    11 => slots[0] = Tag::Int(3), _ => unreachable!(),
                }
            }
        }
        raws.push(raw);
    }
    let mut states = Vec::new();
    for mode in 0..9 {
        let mut next = base.clone();
        match mode {
            0 => {}, 1 => next.redstone = true, 2 => next.tank.amount = 500,
            3 => next.slots[0] = None, 4 => next.slots[0].as_mut().unwrap().count = 1,
            5 => next.slots[1] = Some(ItemStack::new("minecraft:diamond", 1)),
            6 => next.slots[0].as_mut().unwrap().count = 0,
            7 => next.slots[0].as_mut().unwrap().count = 100,
            8 => next.tank.amount = 5000, _ => unreachable!(),
        }
        states.push(next);
    }
    let mut cases = 0;
    for raw in &raws { for tank in &states { compare_casting(tank, raw); cases += 1; } }
    assert_eq!(cases, 216);
}

#[test]
fn unknown_utf16_and_float_bits_stay_on_exact_row_identity() {
    let mut state = smeltery(); let mut raw = state.to_nbt(); opaque(&mut raw);
    let Tag::Compound(fields) = &mut raw else { unreachable!() };
    let Some(Tag::List(rows)) = fields.get_mut(&"fluids".into()) else { unreachable!() };
    opaque(&mut rows[0]); let original_row = rows[0].clone();
    state.fluids.reverse(); state.fluids[1].amount = 77;
    compare_smeltery(&state, &raw);
    let result = state.to_nbt_preserving(&raw).unwrap();
    let Tag::List(rows) = result.get("fluids").unwrap() else { unreachable!() };
    let mut expected_row = original_row; set(&mut expected_row, "amount", Tag::Long(77));
    assert_eq!(rows[1], expected_row);
    let Tag::Compound(old) = &raw else { unreachable!() };
    let Tag::Compound(new) = &result else { unreachable!() };
    let key = JavaString(vec![0xd800, 0x78]); assert_eq!(old.get(&key), new.get(&key));
    state.fluids.remove(1);
    assert!(state.to_nbt_preserving(&raw).is_none(), "opaque row removal must still refuse");
}

#[test]
fn casting_rejects_root_and_nested_retained_wire_without_changing_inputs() {
    let raw = casting().to_nbt(); let mut cases = 0;
    for field in 0..5 { for stale in [false, true] {
        let mut state = casting(); let mut child = ItemStack::new("minecraft:stone", 1);
        let mut baseline = child.clone(); if stale { baseline.damage = 3; }
        child.data_mut().wire = Wire::new(baseline, Arc::<[u8]>::from([1, 2, 3]));
        let mut parent = ItemStack::new("minecraft:chest", 1);
        match field {
            0 => parent = child,
            1 => parent.data_mut().container.push((3, child)),
            2 => parent.data_mut().charged.push(child),
            3 => parent.data_mut().pot_decorations = Some(Box::new([None, Some(child), None, None])),
            4 => parent.data_mut().bundle_contents.push(child), _ => unreachable!(),
        }
        state.slots[0] = Some(parent); compare_casting(&state, &raw);
        assert!(state.to_nbt_preserving(&raw).is_none()); cases += 1;
    }}
    assert_eq!(cases, 10);
}

#[test]
fn native_compound_has_last_value_and_sorted_exact_java_keys() {
    // This is the already-decoded native Compound contract, not an assertion
    // about duplicate key occurrences in an unparsed serialized NBT stream.
    let mut raw = casting().to_nbt();
    let Tag::Compound(fields) = &mut raw else { unreachable!() };
    fields.extend([("redstone".into(), Tag::Byte(1)), ("redstone".into(), Tag::Byte(0))]);
    opaque(&mut raw);
    let mut state = casting(); state.redstone = true;
    let result = state.to_nbt_preserving(&raw).unwrap(); compare_casting(&state, &raw);
    let Tag::Compound(fields) = result else { unreachable!() };
    let keys: Vec<_> = fields.keys().cloned().collect();
    let mut sorted = keys.clone(); sorted.sort(); assert_eq!(keys, sorted);
    assert_eq!(fields.get(&JavaString::from("redstone")), Some(&Tag::Byte(1)));
    let _: Compound = fields;
}

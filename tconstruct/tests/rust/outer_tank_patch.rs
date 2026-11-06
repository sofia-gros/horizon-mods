use super::*;
use crate::tconstruct_source_fluid_cannon::Cannon;
use horizon_items::item_data::Wire;
use horizon_world::nbt::JavaString;
use std::sync::Arc;

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
fn wire(item: &mut ItemStack, stale: bool) {
    item.data_mut().wire = Wire::new(item.clone(), Arc::<[u8]>::from([1, 2, 3]));
    if stale { item.data_mut().set_other("minecraft:custom_name", Some(Tag::String("Changed".into()))); }
}
fn compare_cannon(cannon: &Cannon, raw: &Tag) {
    let before = (format!("{cannon:?}"), raw.clone());
    assert_eq!(cannon.tank_to_nbt_preserving(raw), cannon.legacy_tank_to_nbt_preserving(raw));
    assert_eq!((format!("{cannon:?}"), raw.clone()), before);
}
#[test]
fn cannon_complete_old_method_matches_raw_and_state_matrix() {
    let mut base = Cannon::new(); assert!(base.tank.insert("minecraft:water", 1000));
    base.item = Some(ItemStack::new("minecraft:stone", 2));
    let canonical = base.to_nbt();
    let mut raws = vec![canonical.clone(), Tag::End, Tag::Int(1), Tag::List(vec![]), Tag::compound([] as [(&str, Tag); 0])];
    for key in ["Tank", "Fluid", "Items"] { let mut raw = canonical.clone(); set(&mut raw, key, Tag::Int(1)); raws.push(raw); }
    for mode in 0..8 {
        let mut raw = canonical.clone();
        if mode == 0 { opaque(&mut raw); }
        else {
            let Tag::Compound(fields) = &mut raw else { unreachable!() };
            match mode {
                1 => opaque(fields.get_mut(&"tank".into()).unwrap()),
                2 => opaque(fields.get_mut(&"item".into()).unwrap()),
                3 => set(fields.get_mut(&"tank".into()).unwrap(), "fluid", Tag::String(JavaString(vec![0xd800]))),
                4 => set(fields.get_mut(&"tank".into()).unwrap(), "Amount", Tag::Int(1000)),
                5 => set(fields.get_mut(&"tank".into()).unwrap(), "amount", Tag::Int(-1)),
                6 => { fields.insert("item".into(), Tag::Int(0)); },
                7 => { fields.remove(&"item".into()); },
                _ => unreachable!(),
            }
        }
        raws.push(raw);
    }
    let mut cases = 0;
    for mode in 0..8 {
        let mut next = base.clone();
        match mode {
            0 => {}, 1 => next.tank.amount = 500, 2 => next.tank.amount = 2001,
            3 => next.tank.fluid = Some("minecraft:lava".into()), 4 => next.item = None,
            5 => next.item.as_mut().unwrap().count = 1,
            6 => next.item.as_mut().unwrap().damage = 1,
            7 => next.tank.tag = Some(Tag::Int(1)), _ => unreachable!(),
        }
        for raw in &raws { compare_cannon(&next, raw); cases += 1; }
    }
    assert_eq!(cases, 128);
}

#[test]
fn cannon_retained_wire_and_opaque_item_contracts_match() {
    let mut base = Cannon::new(); base.item = Some(ItemStack::new("minecraft:stone", 2));
    let mut raw = base.to_nbt();
    let Tag::Compound(fields) = &mut raw else { unreachable!() };
    opaque(fields.get_mut(&"item".into()).unwrap()); opaque(&mut raw);
    compare_cannon(&base, &raw);
    let saved = base.tank_to_nbt_preserving(&raw).unwrap();
    assert_eq!(saved.get("item"), raw.get("item"));
    let mut cases = 0;
    for nested in [false, true] { for stale in [false, true] {
        let mut next = base.clone();
        if nested {
            let mut child = ItemStack::new("minecraft:stone", 1); wire(&mut child, stale);
            next.item.as_mut().unwrap().data_mut().container = vec![(0, child)];
        } else { wire(next.item.as_mut().unwrap(), stale); }
        compare_cannon(&next, &raw); assert!(next.tank_to_nbt_preserving(&raw).is_none()); cases += 1;
    }}
    assert_eq!(cases, 4);
}

#[test]
fn controller_patch_matches_unknown_and_malformed_values() {
    let mut state = Smeltery::new(); state.tank.capacity = 4000;
    assert!(state.tank.insert("minecraft:water", 1000));
    let mut extended = state.to_nbt(); opaque(&mut extended);
    let raws = [state.to_nbt(), extended, Tag::End, Tag::Int(2), Tag::List(vec![Tag::Int(3)]), Tag::compound([] as [(&str, Tag); 0])];
    let mut cases = 0;
    for mode in 0..4 {
        let mut next = state.clone();
        match mode { 0 => {}, 1 => next.tank.capacity = 1, 2 => next.tank.fluids[0].amount = 0, 3 => next.tank.fluids.clear(), _ => unreachable!() }
        for raw in &raws {
            let before = raw.clone();
            assert_eq!(patch_smeltery_controller_tank(raw, &next), legacy_patch_smeltery_controller_tank(raw, &next));
            assert_eq!(*raw, before); cases += 1;
        }
    }
    assert_eq!(cases, 24);
}

fn configure(game: &mut Game<'_>, creative: bool, fail: bool, mode: u8) {
    game.creative = creative; game.world.fail = fail;
    match mode {
        0 => {}, 1 => opaque(&mut game.world.raw),
        2 => {
            let Tag::Compound(fields) = &mut game.world.raw else { unreachable!() };
            let tank = fields.get_mut(&"tank".into()).unwrap(); opaque(tank);
            let Tag::Compound(fields) = tank else { unreachable!() };
            if let Some(Tag::List(rows)) = fields.get_mut(&"fluids".into()) { for row in rows { opaque(row); } }
        },
        3 => set(&mut game.world.raw, "Tank", Tag::Int(2)),
        4 => game.world.raw = Tag::Int(2), 5 => game.world.formed = false,
        6 => game.world.controller = 0,
        7 | 8 => wire(game.held.as_mut().unwrap(), mode == 8),
        _ => unreachable!(),
    }
}
fn same_slot(a: &Option<ItemStack>, b: &Option<ItemStack>) -> bool {
    match (a, b) { (None, None) => true, (Some(a), Some(b)) => pluto_stack_storage::same_item_snapshot(a, b), _ => false }
}
#[test]
fn drain_complete_route_matches_saved_and_published_results() {
    let mut cases = 0;
    for item in ["minecraft:water_bucket", "minecraft:bucket", "minecraft:stone", "minecraft:glass_bottle"] {
        for amount in [0, 1000, 2000, 4000] { for creative in [false, true] { for fail in [false, true] { for mode in 0..9 {
            let mut actual = fixture(item, amount); let mut old = fixture(item, amount);
            configure(&mut actual, creative, fail, mode); configure(&mut old, creative, fail, mode);
            assert_eq!(interact_drain(&mut actual, (-2, 0, 0)), legacy_interact_drain(&mut old, (-2, 0, 0)));
            assert_eq!(actual.world.raw, old.world.raw); assert_eq!(actual.world.data, old.world.data);
            assert_eq!(actual.world.writes, old.world.writes); assert_eq!(actual.menus, old.menus); assert_eq!(actual.updates, old.updates);
            assert!(same_slot(&actual.held, &old.held)); assert!(same_slot(&actual.carried, &old.carried));
            assert_eq!(actual.gifts.len(), old.gifts.len());
            for (a, b) in actual.gifts.iter().zip(&old.gifts) { assert!(pluto_stack_storage::same_item_snapshot(a, b)); }
            cases += 1;
        }}}}
    }
    assert_eq!(cases, 576);
}

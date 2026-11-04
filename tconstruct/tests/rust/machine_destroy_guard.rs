#![allow(dead_code)]
#[path = "../../mixins/tconstruct/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path = "../../mixins/tconstruct/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path = "../../mixins/tconstruct/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path = "../../mixins/tconstruct/source-smeltery-state.rs"] mod tconstruct_source_smeltery_state;
#[path = "../../mixins/tconstruct/source-smeltery-layout.rs"] mod tconstruct_source_smeltery_layout;
#[path = "../../mixins/tconstruct/source-smeltery-page.rs"] mod tconstruct_source_smeltery_page;
#[path = "../../mixins/tconstruct/source-machine-destroy-guard.rs"] mod guard;
use horizon_world::nbt::Tag;
fn outer(kind: &str, value: Tag) -> Tag {
    Tag::compound([("id", Tag::String(kind.into())), ("x", Tag::Int(0)), ("y", Tag::Int(64)), ("z", Tag::Int(0)), (kind, value)])
}
#[test] fn canonical_machines_remain_breakable() {
    let melter = outer("tconstruct:melter", tconstruct_source_melter_state::Melter::new().to_nbt());
    let smeltery = outer("tconstruct:smeltery", tconstruct_source_smeltery_state::Smeltery::new().to_nbt());
    assert!(!guard::protect("tconstruct:melter", &melter));
    assert!(!guard::protect("tconstruct:smeltery", &smeltery));
}
#[test] fn unknown_or_noncanonical_data_refuses_player_removal() {
    for (kind, value) in [("tconstruct:melter", tconstruct_source_melter_state::Melter::new().to_nbt()),
                          ("tconstruct:smeltery", tconstruct_source_smeltery_state::Smeltery::new().to_nbt())] {
        let mut future = value.clone(); if let Tag::Compound(fields) = &mut future { fields.insert("future".into(), Tag::Int(7)); }
        assert!(guard::protect(kind, &outer(kind, future)));
        let mut bad_outer = outer(kind, value); if let Tag::Compound(fields) = &mut bad_outer { fields.insert("other_mod".into(), Tag::Int(1)); }
        assert!(guard::protect(kind, &bad_outer));
        assert!(guard::protect(kind, &outer(kind, Tag::Int(5))));
    }
}
#[test] fn unknown_tank_extension_is_protected() {
    let mut value = tconstruct_source_melter_state::Melter::new().to_nbt();
    if let Tag::Compound(fields) = &mut value { if let Some(Tag::Compound(tank)) = fields.get_mut(&"tank".into()) { tank.insert("future".into(), Tag::Int(1)); } }
    assert!(guard::protect("tconstruct:melter", &outer("tconstruct:melter", value)));
}

#[test] fn invalid_utf16_and_malformed_display_are_protected() {
    let mut value = tconstruct_source_melter_state::Melter::new().to_nbt();
    if let Tag::Compound(fields) = &mut value {
        if let Some(Tag::Compound(tank)) = fields.get_mut(&"tank".into()) {
            tank.insert("fluid".into(), Tag::String(horizon_world::nbt::JavaString(vec![0xd800])));
            tank.insert("amount".into(), Tag::Int(1));
        }
    }
    assert!(guard::protect("tconstruct:melter", &outer("tconstruct:melter", value)));
    let mut value = tconstruct_source_melter_state::Melter::new().to_nbt();
    if let Tag::Compound(fields) = &mut value { fields.insert("heat_codes".into(), Tag::Int(42)); }
    assert!(guard::protect("tconstruct:melter", &outer("tconstruct:melter", value)));
}
#[test] fn canonical_display_metadata_is_allowed() {
    let mut value = tconstruct_source_melter_state::Melter::new().to_nbt();
    if let Tag::Compound(fields) = &mut value { fields.insert("heat_codes".into(), Tag::IntArray(vec![1, 2, 3])); }
    assert!(!guard::protect("tconstruct:melter", &outer("tconstruct:melter", value)));
}
#[test] fn ordinary_fueled_machines_with_items_and_fluids_remain_breakable() {
    use horizon_items::items::ItemStack;
    let mut melter = tconstruct_source_melter_state::Melter::new();
    melter.slots[0] = Some(ItemStack::new("minecraft:iron_ore", 1));
    assert!(melter.tank.insert("tconstruct:molten_iron", 90));
    melter.fuel = 20; melter.fuel_quality = 50; melter.temperature = 1000; melter.rate = 5;
    assert!(!guard::protect("tconstruct:melter", &outer("tconstruct:melter", melter.to_nbt())));
    let mut smeltery = tconstruct_source_smeltery_state::Smeltery::new();
    smeltery.slots = vec![Some(ItemStack::new("minecraft:iron_ore", 1))]; smeltery.progress = vec![3];
    smeltery.tank = tconstruct_source_smeltery_tank::SmelteryTank::new(1000);
    assert!(smeltery.tank.insert("tconstruct:molten_iron", 90));
    smeltery.fuel = 20; smeltery.fuel_quality = 50; smeltery.temperature = 1000; smeltery.rate = 5;
    assert!(!guard::protect("tconstruct:smeltery", &outer("tconstruct:smeltery", smeltery.to_nbt())));
}

#[test] fn opaque_outer_inventory_is_refused_but_canonical_projection_is_allowed() {
    let mut item = match horizon_items::item_nbt::item_to_nbt(&horizon_items::items::ItemStack::new("minecraft:iron_ore", 1)) { Tag::Compound(fields) => fields, _ => unreachable!() };
    item.insert("Slot".into(), Tag::Byte(0));
    let mut saved = outer("tconstruct:melter", tconstruct_source_melter_state::Melter::new().to_nbt());
    if let Tag::Compound(fields) = &mut saved { fields.insert("Items".into(), Tag::List(vec![Tag::Compound(item.clone())])); }
    assert!(!guard::protect("tconstruct:melter", &saved));
    item.insert("future".into(), Tag::Int(2));
    if let Tag::Compound(fields) = &mut saved { fields.insert("Items".into(), Tag::List(vec![Tag::Compound(item)])); }
    assert!(guard::protect("tconstruct:melter", &saved));
    if let Tag::Compound(fields) = &mut saved { fields.insert("Items".into(), Tag::Int(1)); }
    assert!(guard::protect("tconstruct:melter", &saved));
}

#[test] fn live_projection_must_be_represented_in_saved_state() {
    use horizon_items::items::ItemStack;
    let iron = ItemStack::new("minecraft:iron_ore", 1);
    let mut melter = tconstruct_source_melter_state::Melter::new(); melter.slots[0] = Some(iron.clone());
    let raw = outer("tconstruct:melter", melter.to_nbt());
    assert!(guard::live_matches("tconstruct:melter", &raw, &[Some(iron.clone()), None, None], None));
    assert!(!guard::live_matches("tconstruct:melter", &raw, &[Some(ItemStack::new("minecraft:gold_ore", 1)), None, None], None));
    assert!(!guard::live_matches("tconstruct:melter", &raw, &[Some(iron.clone()), None, None, Some(iron.clone())], None));
    let mut smeltery = tconstruct_source_smeltery_state::Smeltery::new();
    smeltery.slots = vec![Some(iron.clone()), None]; smeltery.progress = vec![0, 0];
    let raw = outer("tconstruct:smeltery", smeltery.to_nbt());
    let mut page = vec![None; 54];
    assert!(!guard::live_matches("tconstruct:smeltery", &raw, &page, Some(0)));
    assert!(!guard::live_matches("tconstruct:smeltery", &raw, &page, None));
    page[2] = Some(iron.clone());
    assert!(guard::live_matches("tconstruct:smeltery", &raw, &page, Some(0)));
    page[0] = Some(iron.clone());
    assert!(!guard::live_matches("tconstruct:smeltery", &raw, &page, Some(0)));
    page[0] = None; page[3] = Some(iron.clone());
    assert!(!guard::live_matches("tconstruct:smeltery", &raw, &page, Some(0)));
    page[2] = None;
    assert!(!guard::live_matches("tconstruct:smeltery", &raw, &page, Some(0)));
    assert!(!guard::live_matches("tconstruct:smeltery", &raw, &page, None));
}

#[test] fn serialized_smeltery_page_accepts_high_valid_slot() {
    let mut item = match horizon_items::item_nbt::item_to_nbt(&horizon_items::items::ItemStack::new("minecraft:iron_ore", 1)) { Tag::Compound(fields) => fields, _ => unreachable!() };
    item.insert("Slot".into(), Tag::Byte(53));
    let mut saved = outer("tconstruct:smeltery", tconstruct_source_smeltery_state::Smeltery::new().to_nbt());
    if let Tag::Compound(fields) = &mut saved { fields.insert("Items".into(), Tag::List(vec![Tag::Compound(item)])); }
    assert!(!guard::protect("tconstruct:smeltery", &saved));
}

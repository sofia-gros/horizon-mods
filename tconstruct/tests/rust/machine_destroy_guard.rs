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

mod containers {
    pub enum BlockEntity { Chest(Vec<Option<horizon_items::items::ItemStack>>) }
}
mod decode_fixture {
    use horizon_items::item_nbt::decode_items;
    include!("../../mixins/tconstruct/source-machine-decode.rs");
    pub fn decode(tag: &horizon_world::nbt::Tag) -> Vec<Option<horizon_items::items::ItemStack>> {
        let std::ops::ControlFlow::Break(Some((_, crate::containers::BlockEntity::Chest(slots)))) = tconstruct_machine_decode(tag) else { panic!("decode failed") };
        slots
    }
}

extern crate self as horizon_client;
use guard as tconstruct_source_machine_destroy_guard;
#[path = "../../mixins/tconstruct/source-smeltery-nbt-preserve.rs"] mod tconstruct_source_smeltery_nbt_preserve;
#[path = "../../mixins/tconstruct/source-melter-store.rs"] mod tconstruct_source_melter_store;
#[path = "../../mixins/tconstruct/source-machine-projection-world.rs"] mod projection_world;
mod tconstruct_world {
    use std::{collections::{HashMap, HashSet}, sync::Arc, ops::{Deref, DerefMut}};
    use horizon_world::nbt::Tag;
    type Pos = (i32,i32,i32);
    #[derive(Clone, Default)] pub struct Saved { pub block_entity_sources: Vec<(Pos,Tag)> }
    #[derive(Clone, Default)] pub struct Chunk { pub saved: Option<Arc<Saved>> }
    #[derive(Default)] pub struct Chunks {
        pub map: HashMap<(i32,i32), Arc<Chunk>>,
        pub pages: HashMap<Pos, (usize,usize)>,
        pub containers: HashMap<Pos, ([Option<horizon_items::items::ItemStack>;2], u8)>,
    }
    impl Deref for Chunks { type Target = HashMap<(i32,i32), Arc<Chunk>>; fn deref(&self) -> &Self::Target { &self.map } }
    impl DerefMut for Chunks { fn deref_mut(&mut self) -> &mut Self::Target { &mut self.map } }
    impl Chunks {
        pub fn __horizon_mixins_tconstruct_smeltery_ui_pages(&self) -> &HashMap<Pos, (usize,usize)> { &self.pages }
        pub fn __horizon_mixins_tconstruct_smeltery_local_containers(&self) -> &HashMap<Pos, ([Option<horizon_items::items::ItemStack>;2], u8)> { &self.containers }
    }
    #[derive(Default)] pub struct Entities(HashMap<Pos, crate::containers::BlockEntity>);
    impl Deref for Entities { type Target = HashMap<Pos, crate::containers::BlockEntity>; fn deref(&self) -> &Self::Target { &self.0 } }
    impl DerefMut for Entities { fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 } }
    impl Entities { pub fn get_or_insert_with(&mut self, pos: Pos, f: impl FnOnce() -> crate::containers::BlockEntity) -> &mut crate::containers::BlockEntity { self.0.entry(pos).or_insert_with(f) } }
    #[derive(Default)] pub struct World {
        pub chunks: Chunks,
        pub modified: HashSet<(i32,i32)>,
        pub block_entities: Entities,
    }
}


fn fixture(count: usize, offset: Option<usize>) -> (tconstruct_world::World, tconstruct_source_smeltery_state::Smeltery) {
    use horizon_items::items::ItemStack;
    let mut state = tconstruct_source_smeltery_state::Smeltery::new();
    state.slots = vec![None; count]; state.progress = vec![0; count];
    state.slots[count - 1] = Some(ItemStack::new("minecraft:iron_ore", 1));
    let live = tconstruct_source_smeltery_page::project(&state.slots, offset.unwrap_or(0));
    let mut raw = outer("tconstruct:smeltery", state.to_nbt());
    if let Tag::Compound(fields) = &mut raw { fields.insert("Items".into(), horizon_items::item_nbt::encode_items(&live)); }
    let mut world = tconstruct_world::World::default();
    world.chunks.insert((0,0), std::sync::Arc::new(tconstruct_world::Chunk {
        saved: Some(std::sync::Arc::new(tconstruct_world::Saved { block_entity_sources: vec![((0,64,0), raw)] })) }));
    world.block_entities.insert((0,64,0), containers::BlockEntity::Chest(live));
    if let Some(offset) = offset { world.chunks.pages.insert((0,64,0), (count, offset)); }
    (world, state)
}
fn raw(world: &tconstruct_world::World) -> Tag { tconstruct_source_melter_store::read_outer(world, (0,64,0)).unwrap() }
fn live(world: &tconstruct_world::World) -> &[Option<horizon_items::items::ItemStack>] {
    let containers::BlockEntity::Chest(slots) = world.block_entities.get(&(0,64,0)).unwrap(); slots
}

#[test] fn verified_close_allows_filled_controller_after_page_offset_is_discarded() {
    let (world, _) = fixture(71, Some(8));
    let mut view = live(&world).to_vec();
    assert!(guard::close_projection(&raw(&world), &mut view, Some(8), &[None, None]));
    let retired = guard::retire_outer(&raw(&world)).unwrap();
    assert!(guard::live_matches("tconstruct:smeltery", &retired, &view, None));
    assert!(view.iter().all(Option::is_none));
}
#[test] fn close_never_erases_mismatched_or_unknown_page() {
    let (world, _) = fixture(1, Some(0));
    let mut view = live(&world).to_vec(); view[53] = Some(horizon_items::items::ItemStack::new("minecraft:diamond", 1));
    let before = view.clone();
    assert!(!guard::close_projection(&raw(&world), &mut view, Some(0), &[None, None]));
    assert_eq!(view, before);
}
#[test] fn closed_guest_or_runtime_update_retires_cache_in_same_store_transaction() {
    let (mut world, mut state) = fixture(1, None);
    assert!(projection_world::known(&world, (0,64,0)));
    state.slots[0] = None;
    assert!(tconstruct_source_melter_store::write(&mut world, (0,64,0), "tconstruct:smeltery", state.to_nbt()));
    assert!(live(&world).iter().all(Option::is_none));
    assert!(guard::retired_projection(&raw(&world)));
    assert!(guard::live_matches("tconstruct:smeltery", &raw(&world), live(&world), None));
}
#[test] fn open_guest_or_runtime_update_keeps_local_page_current_before_close() {
    let (mut world, mut state) = fixture(71, Some(8));
    let bucket = Some(horizon_items::items::ItemStack::new("minecraft:bucket", 1));
    world.chunks.containers.insert((0,64,0), ([bucket.clone(), None], 0));
    let containers::BlockEntity::Chest(view) = world.block_entities.get_mut(&(0,64,0)).unwrap(); view[0] = bucket.clone();
    state.slots[70] = Some(horizon_items::items::ItemStack::new("minecraft:gold_ore", 1));
    assert!(tconstruct_source_melter_store::write(&mut world, (0,64,0), "tconstruct:smeltery", state.to_nbt()));
    assert_eq!(live(&world)[40], state.slots[70]);
    assert_eq!(live(&world)[0], bucket);
    let mut view = live(&world).to_vec();
    assert!(guard::close_projection(&raw(&world), &mut view, Some(8), &[bucket, None]));
    assert!(view.iter().all(Option::is_none));
}
#[test] fn writer_cannot_erase_unexplained_or_retained_wire_live_items() {
    for wire in [false, true] {
        let (mut world, mut state) = fixture(1, Some(0));
        let containers::BlockEntity::Chest(view) = world.block_entities.get_mut(&(0,64,0)).unwrap();
        if wire {
            let decoded = view[2].as_ref().unwrap().clone();
            view[2].as_mut().unwrap().data_mut().wire = horizon_items::item_data::Wire::new(decoded, std::sync::Arc::from([7u8]));
        } else { view[53] = Some(horizon_items::items::ItemStack::new("minecraft:diamond", 1)); }
        let source_before = raw(&world); let live_before = live(&world).to_vec();
        assert!(!projection_world::known(&world, (0,64,0)));
        state.slots[0] = None;
        assert!(!tconstruct_source_melter_store::write(&mut world, (0,64,0), "tconstruct:smeltery", state.to_nbt()));
        assert!(!projection_world::record(&mut world, (0,64,0), 0));
        assert_eq!(raw(&world), source_before); assert_eq!(live(&world), live_before);
    }
}
#[test] fn forged_valid_shaped_snapshot_does_not_authorize_diamond_deletion() {
    let (mut world, mut state) = fixture(1, None);
    state.slots[0] = None;
    let mut view = vec![None; 54]; view[2] = Some(horizon_items::items::ItemStack::new("minecraft:diamond", 1));
    let mut forged = outer("tconstruct:smeltery", state.to_nbt());
    if let Tag::Compound(fields) = &mut forged {
        fields.insert("Items".into(), horizon_items::item_nbt::encode_items(&view));
        let encoded = view.iter().map(|item| item.as_ref().map(horizon_items::item_nbt::item_to_nbt).unwrap_or_else(|| Tag::Compound(Default::default()))).collect();
        fields.insert("tconstruct:retired_projection".into(), Tag::compound([("version", Tag::Int(1)), ("view", Tag::List(encoded))]));
    }
    assert!(guard::protect("tconstruct:smeltery", &forged));
    assert!(!guard::live_matches("tconstruct:smeltery", &forged, &view, None));
    if let Tag::Compound(fields) = &mut forged { fields.insert("tconstruct:retired_projection".into(), Tag::Int(1)); fields.insert("Items".into(), Tag::List(vec![])); }
    assert!(!guard::live_matches("tconstruct:smeltery", &forged, &view, None));
    let previous = raw(&world); assert!(tconstruct_source_melter_store::replace_outer(&mut world, (0,64,0), &previous, forged));
    world.block_entities.insert((0,64,0), containers::BlockEntity::Chest(view.clone()));
    assert!(!tconstruct_source_melter_store::write(&mut world, (0,64,0), "tconstruct:smeltery", state.to_nbt()));
    assert_eq!(live(&world), view);
}

#[test] fn a_proposed_update_matching_injected_live_data_is_not_old_view_evidence() {
    let (mut world, mut state) = fixture(1, Some(0));
    let diamond = Some(horizon_items::items::ItemStack::new("minecraft:diamond", 1));
    let containers::BlockEntity::Chest(view) = world.block_entities.get_mut(&(0,64,0)).unwrap(); view[2] = diamond.clone();
    state.slots[0] = diamond;
    let before = raw(&world);
    assert!(!tconstruct_source_melter_store::write(&mut world, (0,64,0), "tconstruct:smeltery", state.to_nbt()));
    assert_eq!(raw(&world), before);
    assert_eq!(live(&world)[2], state.slots[0]);
}
#[test] fn batch_failure_leaves_source_live_and_neighbor_unchanged() {
    let (mut world, mut state) = fixture(1, Some(0));
    let before = raw(&world); let page = live(&world).to_vec(); state.slots[0] = None;
    let changes = [((0,64,0), "tconstruct:smeltery", state.to_nbt()), ((32,64,0), "tconstruct:melter", Tag::Int(1))];
    assert!(!tconstruct_source_melter_store::write_many(&mut world, &changes));
    assert_eq!(raw(&world), before); assert_eq!(live(&world), page);
}
#[test] fn closed_and_open_pages_survive_real_host_overlay_nbt_and_decode() {
    use horizon_world::nbt::{self, NamedTag, Limits};
    for open in [false, true] {
        let (mut world, mut state) = fixture(71, Some(8));
        if !open { world.chunks.pages.clear(); }
        state.slots[0] = Some(horizon_items::items::ItemStack::new("minecraft:gold_ore", 1));
        assert!(tconstruct_source_melter_store::write(&mut world, (0,64,0), "tconstruct:smeltery", state.to_nbt()));
        let original = raw(&world);
        let mut encoded = outer("tconstruct:smeltery", state.to_nbt());
        if let Tag::Compound(fields) = &mut encoded {
            fields.remove(&"tconstruct:smeltery".into());
            fields.insert("Items".into(), horizon_items::item_nbt::encode_items(live(&world)));
            fields.insert("keepPacked".into(), Tag::Byte(0));
            fields.insert("components".into(), Tag::Compound(Default::default()));
        }
        let merged = horizon_client_core::block_entity_nbt::overlay(&original, &encoded, &encoded);
        let merged = horizon_client_core::block_entity_nbt::canonical(&merged).unwrap_or(merged);
        let bytes = nbt::write_named(&NamedTag { name: "".into(), tag: merged }, Limits::saved()).unwrap();
        let restored = nbt::read_named(&bytes, Limits::saved()).unwrap().0.tag;
        let decoded = decode_fixture::decode(&restored);
        assert_eq!(decoded.len(), 54);
        assert!(!guard::protect("tconstruct:smeltery", &restored));
        assert!(guard::live_matches("tconstruct:smeltery", &restored, &decoded, None));
        assert!(guard::live_matches("tconstruct:smeltery", &restored, &decoded, None));
    }
}
#[test] fn loaded_bucket_slots_and_unsupported_host_metadata_remain_protected() {
    let (world, _) = fixture(1, Some(0));
    let mut tag = raw(&world); let mut page = live(&world).to_vec();
    page[0] = Some(horizon_items::items::ItemStack::new("minecraft:bucket", 1));
    if let Tag::Compound(fields) = &mut tag { fields.insert("Items".into(), horizon_items::item_nbt::encode_items(&page)); }
    assert!(!guard::current_projection(&tag, &page, None, &[None, None]));
    for (key, value) in [("keepPacked", Tag::Byte(1)), ("components", Tag::compound([("unknown", Tag::Int(1))])), ("tconstruct:retired_projection", Tag::Int(2))] {
        let mut changed = raw(&world);
        if let Tag::Compound(fields) = &mut changed { fields.insert(key.into(), value); }
        assert!(guard::protect("tconstruct:smeltery", &changed));
        assert!(!guard::current_projection(&changed, live(&world), Some(0), &[None, None]));
    }
}
#[test] fn single_item_menu_transaction_restores_old_page_before_store_commit() {
    let (mut world, mut state) = fixture(1, Some(0));
    let expected = live(&world).to_vec();
    state.slots[0] = Some(horizon_items::items::ItemStack::new("minecraft:gold_ore", 1));
    let changed = tconstruct_source_smeltery_page::project(&state.slots, 0);
    // Simulate menu_ops output, then the actual adapter's restoration before
    // calling the real source writer. The writer commits the new page itself.
    world.block_entities.insert((0,64,0), containers::BlockEntity::Chest(changed.clone()));
    world.block_entities.insert((0,64,0), containers::BlockEntity::Chest(expected));
    assert!(tconstruct_source_melter_store::write(&mut world, (0,64,0), "tconstruct:smeltery", state.to_nbt()));
    assert_eq!(live(&world), changed);
    assert!(projection_world::record(&mut world, (0,64,0), 0));
}

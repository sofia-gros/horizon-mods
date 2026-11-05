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

#[test] fn verified_close_allows_filled_controller_after_page_offset_is_discarded() {
    use horizon_items::items::ItemStack;
    let mut state = tconstruct_source_smeltery_state::Smeltery::new();
    state.slots = vec![None; 71]; state.progress = vec![0; 71];
    state.slots[0] = Some(ItemStack::new("minecraft:iron_ore", 1));
    state.slots[70] = Some(ItemStack::new("minecraft:gold_ore", 1));
    let raw = outer("tconstruct:smeltery", state.to_nbt());
    let containers = [Some(ItemStack::new("minecraft:bucket", 1)), None];
    let mut live = tconstruct_source_smeltery_page::project_with_containers(&state.slots, 8, &containers);
    let proof = guard::close_projection(&raw, &mut live, Some(8), &containers);
    assert!(proof);
    assert!(live.iter().all(Option::is_none));
    // Both controller block variants route through this same smeltery state.
    assert!(guard::live_matches_with_closed_projection("tconstruct:smeltery", &raw, &live, None, proof));
    assert!(!guard::live_matches_with_closed_projection("tconstruct:smeltery", &raw, &live, None, false));
    // Returning containers belongs to the caller; the original slots are not changed.
    assert_eq!(containers[0].as_ref().unwrap().count, 1);
    // Closed runtime, hopper and guest writes may change authoritative inputs
    // without reintroducing the retired UI page or its old page offset.
    state.slots[0] = None;
    state.slots[4] = Some(ItemStack::new("minecraft:copper_ore", 1));
    let current = outer("tconstruct:smeltery", state.to_nbt());
    assert!(guard::live_matches_with_closed_projection("tconstruct:smeltery", &current, &live, None, proof));
    // A later unexplained live insertion must still prevent destruction.
    live[20] = Some(ItemStack::new("minecraft:diamond", 1));
    assert!(!guard::live_matches_with_closed_projection("tconstruct:smeltery", &current, &live, None, proof));
}

#[test] fn close_never_erases_mismatched_or_unknown_page() {
    use horizon_items::items::ItemStack;
    let mut state = tconstruct_source_smeltery_state::Smeltery::new();
    state.slots = vec![Some(ItemStack::new("minecraft:iron_ore", 1))]; state.progress = vec![0];
    let raw = outer("tconstruct:smeltery", state.to_nbt());
    let page = tconstruct_source_smeltery_page::project(&state.slots, 0);
    for (mut live, offset) in [(page.clone(), None), (vec![None; 54], Some(0)), (vec![None; 27], Some(0))] {
        let before = live.clone();
        assert!(!guard::close_projection(&raw, &mut live, offset, &[None, None]));
        assert_eq!(live, before);
    }
    let mut live = page.clone(); live[0] = Some(ItemStack::new("minecraft:bucket", 1));
    let before = live.clone();
    assert!(!guard::close_projection(&raw, &mut live, Some(0), &[None, None]));
    assert_eq!(live, before);
    let mut opaque = raw.clone();
    if let Tag::Compound(fields) = &mut opaque { fields.insert("future".into(), Tag::Int(7)); }
    let mut live = page.clone();
    assert!(!guard::close_projection(&opaque, &mut live, Some(0), &[None, None]));
    assert_eq!(live, page);
    assert!(guard::protect("tconstruct:smeltery", &opaque));
    // An ephemeral proof is not inferred after reload or from a 27-slot chest.
    assert!(!guard::live_matches_with_closed_projection("tconstruct:smeltery", &raw, &[None; 0], None, true));
    assert!(!guard::live_matches_with_closed_projection("tconstruct:smeltery", &raw, &vec![None; 27], None, true));
}

// Exercise the actual decode adapter with the real host item codec. Only the
// enclosing BlockEntity enum is reduced; no UI, world or networking is mocked
// into a claim of an end-to-end pass.
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

#[test] fn canonical_host_envelope_is_allowed_but_nonempty_extensions_are_protected() {
    let mut raw = outer("tconstruct:smeltery", tconstruct_source_smeltery_state::Smeltery::new().to_nbt());
    if let Tag::Compound(fields) = &mut raw {
        fields.insert("keepPacked".into(), Tag::Byte(0));
        fields.insert("components".into(), Tag::Compound(Default::default()));
    }
    assert!(!guard::protect("tconstruct:smeltery", &raw));
    for (key, value) in [("keepPacked", Tag::Byte(1)), ("keepPacked", Tag::Int(0)),
                         ("components", Tag::compound([("future", Tag::Int(1))])),
                         ("tconstruct:retired_projection", Tag::Int(2))] {
        let mut unknown = raw.clone();
        if let Tag::Compound(fields) = &mut unknown { fields.insert(key.into(), value); }
        assert!(guard::protect("tconstruct:smeltery", &unknown));
        assert!(guard::retire_outer(&unknown).is_none());
    }
}

#[test] fn retired_provenance_survives_real_nbt_save_overlay_and_reload() {
    use horizon_items::{items::ItemStack, item_nbt::encode_items};
    use horizon_world::nbt::{self, NamedTag, Limits};
    let mut state = tconstruct_source_smeltery_state::Smeltery::new();
    state.slots = vec![None; 71]; state.progress = vec![0; 71];
    state.slots[0] = Some(ItemStack::new("minecraft:iron_ore", 1));
    state.slots[70] = Some(ItemStack::new("minecraft:gold_ore", 1));
    let original = outer("tconstruct:smeltery", state.to_nbt());
    let mut live = tconstruct_source_smeltery_page::project(&state.slots, 8);
    assert!(guard::close_projection(&original, &mut live, Some(8), &[None, None]));
    let retired = guard::retire_outer(&original).unwrap();
    assert!(guard::retired_projection(&retired));
    // These are the ordinary Chest envelope fields produced by the host.
    let encoded = Tag::compound([("id", Tag::String("tconstruct:smeltery".into())),
        ("x", Tag::Int(0)), ("y", Tag::Int(64)), ("z", Tag::Int(0)),
        ("keepPacked", Tag::Byte(0)), ("components", Tag::Compound(Default::default())),
        ("Items", encode_items(&live))]);
    // Use the real host's overlay and canonicalization, not an imitation of
    // unknown-field retention. This custom id is intentionally not canonicalized.
    let merged = horizon_client_core::block_entity_nbt::overlay(&retired, &encoded, &encoded);
    let saved = horizon_client_core::block_entity_nbt::canonical(&merged).unwrap_or(merged);
    let bytes = nbt::write_named(&NamedTag { name: "".into(), tag: saved }, Limits::saved()).unwrap();
    let (restored, used) = nbt::read_named(&bytes, Limits::saved()).unwrap();
    assert_eq!(used, bytes.len());
    let mut raw = restored.tag;
    let live = decode_fixture::decode(&raw);
    assert_eq!(live.len(), 54);
    assert!(guard::retired_projection(&raw));
    assert!(!guard::protect("tconstruct:smeltery", &raw));
    assert!(guard::live_matches("tconstruct:smeltery", &raw, &live, None));
    // Guest/runtime authoritative slot updates preserve the outer provenance;
    // the retired live view stays empty and is not counted a second time.
    state.slots[0] = None;
    state.slots[3] = Some(ItemStack::new("minecraft:copper_ore", 1));
    if let Tag::Compound(fields) = &mut raw { fields.insert("tconstruct:smeltery".into(), state.to_nbt()); }
    assert!(guard::live_matches("tconstruct:smeltery", &raw, &live, None));
    let opened = guard::open_outer(&raw).unwrap();
    assert!(!guard::retired_projection(&opened));
    assert!(!guard::live_matches("tconstruct:smeltery", &opened, &live, None));
}

#[test] fn persisted_proof_never_trusts_injected_items_or_unknown_version() {
    use horizon_items::items::ItemStack;
    let mut state = tconstruct_source_smeltery_state::Smeltery::new();
    state.slots = vec![Some(ItemStack::new("minecraft:iron_ore", 1))]; state.progress = vec![0];
    let raw = guard::retire_outer(&outer("tconstruct:smeltery", state.to_nbt())).unwrap();
    let mut live = vec![None; 54];
    live[53] = Some(ItemStack::new("minecraft:diamond", 1));
    assert!(!guard::live_matches("tconstruct:smeltery", &raw, &live, None));
    let mut injected = raw.clone();
    if let Tag::Compound(fields) = &mut injected { fields.insert("Items".into(), horizon_items::item_nbt::encode_items(&live)); }
    assert!(!guard::retired_projection(&injected));
    assert!(!guard::live_matches("tconstruct:smeltery", &injected, &vec![None; 54], None));
    if let Tag::Compound(fields) = &mut injected { fields.insert("tconstruct:retired_projection".into(), Tag::Int(2)); }
    assert!(guard::protect("tconstruct:smeltery", &injected));
    assert!(guard::open_outer(&injected).is_none());
}

#[test] fn smeltery_decode_keeps_high_serialized_slots_instead_of_truncating_them() {
    use horizon_items::items::ItemStack;
    let mut live = vec![None; 54];
    live[53] = Some(ItemStack::new("minecraft:diamond", 1));
    let mut raw = outer("tconstruct:smeltery", tconstruct_source_smeltery_state::Smeltery::new().to_nbt());
    if let Tag::Compound(fields) = &mut raw { fields.insert("Items".into(), horizon_items::item_nbt::encode_items(&live)); }
    let decoded = decode_fixture::decode(&raw);
    assert_eq!(decoded, live);
    assert!(!guard::live_matches("tconstruct:smeltery", &raw, &decoded, None));
}

#[test] fn active_snapshot_allows_reload_without_trusting_later_live_mutations() {
    use horizon_items::{items::ItemStack, item_nbt::encode_items};
    use horizon_world::nbt::{self, NamedTag, Limits};
    let mut state = tconstruct_source_smeltery_state::Smeltery::new();
    state.slots = vec![None; 71]; state.progress = vec![0; 71];
    state.slots[70] = Some(ItemStack::new("minecraft:iron_ore", 1));
    let live = tconstruct_source_smeltery_page::project(&state.slots, 8);
    let raw = outer("tconstruct:smeltery", state.to_nbt());
    let mut snapshot = guard::snapshot_outer(&raw, &live, 8, &[None, None]).unwrap();
    // Save while the scrolled menu is still open, then load with no UI offset.
    if let Tag::Compound(fields) = &mut snapshot { fields.insert("Items".into(), encode_items(&live)); }
    let bytes = nbt::write_named(&NamedTag { name: "".into(), tag: snapshot }, Limits::saved()).unwrap();
    let mut restored = nbt::read_named(&bytes, Limits::saved()).unwrap().0.tag;
    let loaded_live = decode_fixture::decode(&restored);
    assert!(!guard::protect("tconstruct:smeltery", &restored));
    assert!(guard::live_matches("tconstruct:smeltery", &restored, &loaded_live, None));
    // The machine can consume an input while its closed cache remains old.
    // That old cache must not be dropped as a second independent inventory.
    state.slots[70] = None;
    if let Tag::Compound(fields) = &mut restored { fields.insert("tconstruct:smeltery".into(), state.to_nbt()); }
    assert!(guard::live_matches("tconstruct:smeltery", &restored, &loaded_live, None));
    let mut unexpected = loaded_live.clone();
    unexpected[53] = Some(ItemStack::new("minecraft:diamond", 1));
    assert!(!guard::live_matches("tconstruct:smeltery", &restored, &unexpected, None));
    unexpected = loaded_live.clone(); unexpected[40] = None;
    assert!(!guard::live_matches("tconstruct:smeltery", &restored, &unexpected, None));
    // Removing provenance before a new local menu prevents trusting the old
    // snapshot for newly modified live items.
    let opened = guard::open_outer(&restored).unwrap();
    assert!(!guard::live_matches("tconstruct:smeltery", &opened, &loaded_live, None));
}

#[test] fn active_snapshot_never_discards_pending_containers_after_reload() {
    use horizon_items::items::ItemStack;
    let mut state = tconstruct_source_smeltery_state::Smeltery::new();
    state.slots = vec![Some(ItemStack::new("minecraft:iron_ore", 1))]; state.progress = vec![0];
    let containers = [Some(ItemStack::new("minecraft:bucket", 1)), None];
    let live = tconstruct_source_smeltery_page::project_with_containers(&state.slots, 0, &containers);
    let raw = outer("tconstruct:smeltery", state.to_nbt());
    let snapshot = guard::snapshot_outer(&raw, &live, 0, &containers).unwrap();
    assert!(!guard::protect("tconstruct:smeltery", &snapshot));
    // Even when the local/guest container registry is gone after reload, the
    // snapshot knows that these items are not part of the saved melting slots.
    assert!(!guard::live_matches("tconstruct:smeltery", &snapshot, &live, None));
    let mut lost_bucket = live.clone(); lost_bucket[0] = None;
    assert!(!guard::live_matches("tconstruct:smeltery", &snapshot, &lost_bucket, None));
}

#[test] fn active_snapshot_cannot_be_created_for_unknown_or_mismatched_live_data() {
    use horizon_items::items::ItemStack;
    let mut state = tconstruct_source_smeltery_state::Smeltery::new();
    state.slots = vec![Some(ItemStack::new("minecraft:iron_ore", 1))]; state.progress = vec![0];
    let raw = outer("tconstruct:smeltery", state.to_nbt());
    let mut live = tconstruct_source_smeltery_page::project(&state.slots, 0);
    live[53] = Some(ItemStack::new("minecraft:diamond", 1));
    assert!(guard::snapshot_outer(&raw, &live, 0, &[None, None]).is_none());
    let clean = tconstruct_source_smeltery_page::project(&state.slots, 0);
    let mut live = clean.clone();
    let decoded = live[2].as_ref().unwrap().clone();
    live[2].as_mut().unwrap().data_mut().wire = horizon_items::item_data::Wire::new(decoded, std::sync::Arc::from([7u8]));
    assert!(guard::snapshot_outer(&raw, &live, 0, &[None, None]).is_none());
    // A later edit makes wire_patch() return None, but retained bytes still
    // cannot be silently discarded when recording or retiring a projection.
    live[2].as_mut().unwrap().damage = 1;
    assert!(live[2].as_ref().unwrap().wire_patch().is_none());
    state.slots[0].as_mut().unwrap().damage = 1;
    let damaged = outer("tconstruct:smeltery", state.to_nbt());
    assert!(guard::snapshot_outer(&damaged, &live, 0, &[None, None]).is_none());
    let mut snapshot = guard::snapshot_outer(&raw, &clean, 0, &[None, None]).unwrap();
    if let Tag::Compound(fields) = &mut snapshot {
        if let Some(Tag::Compound(marker)) = fields.get_mut(&"tconstruct:retired_projection".into()) {
            marker.insert("future".into(), Tag::Int(1));
        }
    }
    assert!(guard::protect("tconstruct:smeltery", &snapshot));
    assert!(guard::open_outer(&snapshot).is_none());
    assert!(!guard::live_matches("tconstruct:smeltery", &snapshot, &clean, None));
}

// The source-store and recording helper run unchanged against a minimal world
// container. These tests establish staging/CAS semantics, not real LAN or UI.
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
        pub containers: HashMap<Pos, ([Option<horizon_items::items::ItemStack>;2], u8)>,
    }
    impl Deref for Chunks { type Target = HashMap<(i32,i32), Arc<Chunk>>; fn deref(&self) -> &Self::Target { &self.map } }
    impl DerefMut for Chunks { fn deref_mut(&mut self) -> &mut Self::Target { &mut self.map } }
    impl Chunks {
        pub fn __horizon_mixins_tconstruct_smeltery_local_containers(&self) -> &HashMap<Pos, ([Option<horizon_items::items::ItemStack>;2], u8)> { &self.containers }
    }
    #[derive(Default)] pub struct World {
        pub chunks: Chunks,
        pub modified: HashSet<(i32,i32)>,
        pub block_entities: HashMap<Pos, crate::containers::BlockEntity>,
    }
}

#[test] fn recording_and_source_writes_keep_provenance_without_trusting_unsaved_items() {
    use horizon_items::items::ItemStack;
    use tconstruct_source_melter_store as store;
    let pos = (0,64,0);
    let mut world = tconstruct_world::World::default();
    world.chunks.insert((0,0), std::sync::Arc::new(tconstruct_world::Chunk::default()));
    let mut state = tconstruct_source_smeltery_state::Smeltery::new();
    state.slots = vec![Some(ItemStack::new("minecraft:iron_ore", 1))]; state.progress = vec![0];
    assert!(store::write(&mut world, pos, "tconstruct:smeltery", state.to_nbt()));
    let live = tconstruct_source_smeltery_page::project(&state.slots, 0);
    world.block_entities.insert(pos, containers::BlockEntity::Chest(live.clone()));
    assert!(projection_world::record(&mut world, pos, 0));
    let snapshot = store::read_outer(&world, pos).unwrap();
    assert!(guard::live_matches("tconstruct:smeltery", &snapshot, &live, None));
    // The real store writer used by guest/runtime edits preserves outer proof.
    state.slots[0] = None;
    assert!(store::write(&mut world, pos, "tconstruct:smeltery", state.to_nbt()));
    let updated = store::read_outer(&world, pos).unwrap();
    assert!(guard::live_matches("tconstruct:smeltery", &updated, &live, None));
    // A stale close transaction may not overwrite a concurrent source update.
    assert!(!store::replace_outer(&mut world, pos, &snapshot, guard::retire_outer(&snapshot).unwrap()));
    assert_eq!(store::read_outer(&world, pos), Some(updated.clone()));
    let containers::BlockEntity::Chest(changed) = world.block_entities.get_mut(&pos).unwrap();
    changed[53] = Some(ItemStack::new("minecraft:diamond", 1));
    assert!(!projection_world::record(&mut world, pos, 0));
    assert_eq!(store::read_outer(&world, pos), Some(updated.clone()));
    let containers::BlockEntity::Chest(changed) = world.block_entities.get(&pos).unwrap();
    assert!(!guard::live_matches("tconstruct:smeltery", &updated, changed, None));
}

#[test] fn legacy_migration_requires_exact_serialized_and_authoritative_page() {
    use horizon_items::{items::ItemStack, item_nbt::encode_items};
    use tconstruct_source_melter_store as store;
    let mut state = tconstruct_source_smeltery_state::Smeltery::new();
    state.slots = vec![None; 71]; state.progress = vec![0; 71];
    state.slots[0] = Some(ItemStack::new("minecraft:gold_ore", 1));
    state.slots[70] = Some(ItemStack::new("minecraft:iron_ore", 1));
    let live = tconstruct_source_smeltery_page::project(&state.slots, 8);
    let mut raw = outer("tconstruct:smeltery", state.to_nbt());
    assert!(guard::legacy_snapshot_outer(&raw, &live).is_none()); // no serialized proof
    if let Tag::Compound(fields) = &mut raw { fields.insert("Items".into(), encode_items(&live)); }
    assert!(guard::legacy_snapshot_outer(&raw, &live).is_some());
    assert!(guard::live_matches("tconstruct:smeltery", &raw, &live, None));
    let mut injected = live.clone(); injected[53] = Some(ItemStack::new("minecraft:diamond", 1));
    assert!(guard::legacy_snapshot_outer(&raw, &injected).is_none());
    // Merely matching some other source page does not excuse an unsaved edit.
    let other_page = tconstruct_source_smeltery_page::project(&state.slots, 0);
    assert!(guard::legacy_snapshot_outer(&raw, &other_page).is_none());
    let mut malformed = raw.clone();
    if let Tag::Compound(fields) = &mut malformed { fields.insert("Items".into(), encode_items(&injected)); }
    assert!(guard::legacy_snapshot_outer(&malformed, &injected).is_none());
    let pos = (0,64,0);
    let mut world = tconstruct_world::World::default();
    world.chunks.insert((0,0), std::sync::Arc::new(tconstruct_world::Chunk {
        saved: Some(std::sync::Arc::new(tconstruct_world::Saved { block_entity_sources: vec![(pos, raw)] })) }));
    world.block_entities.insert(pos, containers::BlockEntity::Chest(live.clone()));
    assert!(projection_world::capture_legacy(&mut world, pos));
    state.slots[70] = None;
    assert!(store::write(&mut world, pos, "tconstruct:smeltery", state.to_nbt()));
    assert!(guard::live_matches("tconstruct:smeltery", &store::read_outer(&world, pos).unwrap(), &live, None));
    assert!(!projection_world::capture_legacy(&mut world, pos)); // never remigrate existing proof
}

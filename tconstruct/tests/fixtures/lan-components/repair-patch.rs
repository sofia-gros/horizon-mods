//! Focused test of the real patch guard and repair commit helper.
use horizon_items::{item_data::{parse_stack, Wire}, items::ItemStack};
use horizon_protocol::{buf::{Codec, Reader, Writer}, item::{component, Component, DelimitedSlot, Stack}};
use std::sync::Arc;
use crate::{fixed_set_damage, repair_from_slot, take_repair, tconstruct_source_tool_normalize::{finalize_tool, patch_safe_for_tool_mutation}};

fn damaged_tool(removal: bool, decorated: bool) -> ItemStack {
    let input: serde_json::Value = serde_json::from_str(include_str!("cases.json")).unwrap();
    let index = usize::from(decorated);
    let mut stack = parse_stack(input["cases"][index]["snbt"].as_str().unwrap()).unwrap().unwrap();
    if decorated { stack.data_mut().set_other("minecraft:custom_name", Some(horizon_world::nbt::Tag::String("Workshop pickaxe".into()))); }
    finalize_tool(&mut stack, |_| None).unwrap();
    fixed_set_damage(&mut stack, 10);
    let data = stack.data().unwrap();
    let tool = data.other("minecraft:tool").unwrap();
    let raw_tool = crate::pluto_tool_codec::encode(tool, &|_, _| None).unwrap();
    let mut wire = Stack::new(1, 1);
    wire.added = vec![
        (component::DAMAGE, Component::Int(10)),
        (component::MAX_DAMAGE, Component::Int(60)),
        (component::CUSTOM_DATA, Component::Nbt(data.other("minecraft:custom_data").unwrap().clone())),
        (28, Component::Raw(raw_tool)),
    ];
    if decorated {
        use horizon_world::nbt::Tag;
        wire.added.push((component::CUSTOM_NAME, Component::Text(data.other("minecraft:custom_name").unwrap().clone())));
        let Some(Tag::List(lore)) = data.other("minecraft:lore") else { panic!("lore missing") };
        wire.added.push((component::LORE, Component::Lore(lore.clone())));
        let Some(Tag::String(model)) = data.other("minecraft:item_model") else { panic!("model missing") };
        wire.added.push((component::ITEM_MODEL, Component::Ident(model.to_string_lossy())));
    }
    if removal { wire.removed.push(105); }
    let mut writer = Writer::new();
    DelimitedSlot::write(&Some(wire), &mut writer);
    let decoded = DelimitedSlot::read(&mut Reader::new(&writer.data)).unwrap().unwrap();
    let patch = decoded.wire.0.unwrap();
    let snapshot = stack.clone();
    stack.data_mut().wire = Wire::new(snapshot, Arc::clone(&patch));
    stack
}

#[test]
fn original_unknown_patch_is_detected_before_damage_invalidates_snapshot() {
    let original = damaged_tool(true, false);
    assert!(original.wire_patch().is_some());
    assert!(!patch_safe_for_tool_mutation(&original));
    let mut changed = original.clone();
    fixed_set_damage(&mut changed, 5);
    assert!(changed.wire_patch().is_none());
    assert!(!patch_safe_for_tool_mutation(&changed));
    assert_eq!(original.damage, 10);
    let slots = [Some(original), Some(ItemStack::new("minecraft:iron_ingot", 3))];
    assert_eq!(slots[0].as_ref().unwrap().damage, 10);
    assert_eq!(slots[1].as_ref().unwrap().count, 3);
}

#[test]
fn known_patch_allows_repair_commit_and_consumes_only_planned_items() {
    let original = damaged_tool(false, false);
    assert!(patch_safe_for_tool_mutation(&original));
    let (used, restored) = repair_from_slot(10, 3, 5.0);
    assert_eq!((used, restored), (2, 10));
    let mut repaired = original.clone();
    fixed_set_damage(&mut repaired, 0);
    finalize_tool(&mut repaired, |_| None).unwrap();
    let mut slots = vec![Some(original), Some(ItemStack::new("minecraft:iron_ingot", 3))];
    let mut carried = None;
    assert!(take_repair(&mut slots, &mut carried, repaired, &[(0, 1), (1, used as u32)]));
    assert!(slots[0].is_none());
    assert_eq!(slots[1].as_ref().unwrap().count, 1);
    assert_eq!(carried.unwrap().damage, 0);
}

#[test]
fn renamed_tool_with_lore_and_model_is_not_rejected() {
    let original = damaged_tool(false, true);
    assert!(patch_safe_for_tool_mutation(&original));
    let mut repaired = original.clone();
    fixed_set_damage(&mut repaired, 5);
    finalize_tool(&mut repaired, |_| None).unwrap();
    let data = repaired.data().unwrap();
    for key in ["minecraft:custom_name", "minecraft:lore", "minecraft:item_model"] {
        assert!(data.other(key).is_some(), "{key}");
    }
}

#[test]
fn cached_patch_is_checked_before_modifier_refill_or_extraction_mutation() {
    use crate::tconstruct_source_tool_normalize::clone_for_tool_mutation;
    use horizon_world::nbt::Tag;
    for operation in ["modifier", "overslime", "extraction"] {
        let original = damaged_tool(false, false);
        let mut changed = clone_for_tool_mutation(&original).expect("known patch accepted");
        let mut custom = changed.data().unwrap().other("minecraft:custom_data").unwrap().clone();
        let Tag::Compound(ref mut fields) = custom else { panic!("expected custom data") };
        fields.insert("fixture_operation".into(), Tag::String(operation.into()));
        changed.data_mut().set_other("minecraft:custom_data", Some(custom));
        assert!(changed.wire_patch().is_none());
        // Rechecking this changed clone would incorrectly reject it.
        assert!(!patch_safe_for_tool_mutation(&changed));
        finalize_tool(&mut changed, |_| None).unwrap();
        let mut slots = vec![Some(original), Some(ItemStack::new("minecraft:iron_ingot", 3))];
        let mut carried = None;
        assert!(take_repair(&mut slots, &mut carried, changed, &[(0, 1), (1, 1)]));
        assert_eq!(slots[1].as_ref().unwrap().count, 2);
        assert!(carried.is_some());
    }
}

#[test]
fn unsupported_patch_is_rejected_before_any_input_mutation() {
    use crate::tconstruct_source_tool_normalize::clone_for_tool_mutation;
    let original = damaged_tool(true, false);
    let slots = [Some(original), Some(ItemStack::new("minecraft:iron_ingot", 3))];
    assert!(clone_for_tool_mutation(slots[0].as_ref().unwrap()).is_none());
    assert_eq!(slots[0].as_ref().unwrap().damage, 10);
    assert_eq!(slots[1].as_ref().unwrap().count, 3);
}

#[test]
fn real_recipe_plans_accept_known_patch_and_unknown_patch_is_retained() {
    use std::collections::{BTreeMap, HashMap, HashSet};
    use crate::{tconstruct_source_overslime, tconstruct_source_tool_normalize::clone_for_tool_mutation, tconstruct_source_upgrade_rules};
    let recipe = br#"{"type":"tconstruct:modifier","allow_crystal":true,"check_trait_level":true,"inputs":[{"item":"minecraft:glowstone"},{"item":"minecraft:daylight_detector"},{"item":"minecraft:shroomlight"}],"level":1,"result":"tconstruct:glowing","slots":{"abilities":1},"tools":{"tag":"tconstruct:modifiable/durability"}}"#.to_vec();
    let recipes = BTreeMap::from([("glowing".to_owned(), recipe)]);
    let tags = HashMap::from([
        ("tconstruct:modifiable".to_owned(), HashSet::from(["tconstruct:pickaxe".to_owned()])),
        ("tconstruct:modifiable/durability".to_owned(), HashSet::from(["tconstruct:pickaxe".to_owned()])),
    ]);
    let inputs = ["minecraft:glowstone", "minecraft:daylight_detector", "minecraft:shroomlight"].map(|id| Some(ItemStack::new(id, 1)));
    let known = damaged_tool(false, true);
    let clone = clone_for_tool_mutation(&known).expect("known patch accepted");
    let upgrade = tconstruct_source_upgrade_rules::plan_with_traits(&recipes, &tags, &clone, &inputs, 10, 1, &BTreeMap::new()).expect("real modifier recipe");
    assert_eq!(upgrade.id, "tconstruct:glowing");
    assert_eq!(upgrade.consumed, vec![(0, 1), (1, 1), (2, 1), (3, 1)]);
    let extraction_inputs = [Some(ItemStack::new("minecraft:wet_sponge", 1)), Some(ItemStack::new("tconstruct:sky_slime_crystal", 1))];
    let extraction = tconstruct_source_upgrade_rules::extract_upgrade_with_traits(&tags, &clone, &extraction_inputs, "tconstruct:reinforced", &BTreeMap::new()).expect("real extraction recipe");
    assert_eq!(extraction.upgrade.amount, 4);
    assert_eq!(tconstruct_source_overslime::refill(12, 50, 10, 3, 0), Some((42, 3)));
    let unknown = damaged_tool(true, true);
    let slots = [Some(unknown), Some(ItemStack::new("minecraft:glowstone", 1))];
    assert!(clone_for_tool_mutation(slots[0].as_ref().unwrap()).is_none());
    assert_eq!(slots[0].as_ref().unwrap().damage, 10);
    assert_eq!(slots[1].as_ref().unwrap().count, 1);
}

#[test]
fn saved_tool_decoder_preserves_unresolved_shape_and_unknown_data() {
    use horizon_items::{item_data::canon_stack, item_nbt::{item_from_nbt, item_to_nbt}};
    let raw = r#"{id:"tconstruct:pickaxe",count:1,components:{"minecraft:tool":{rules:[{blocks:"unresolved_block",speed:2.0f}],"future:field":7},"minecraft:custom_data":{"fixture:unknown":42}}}"#;
    let original = parse_stack(raw).unwrap().unwrap();
    let reloaded = item_from_nbt(&item_to_nbt(&original)).unwrap();
    assert_eq!(canon_stack(Some(&reloaded)), canon_stack(Some(&original)));
    let data = reloaded.data().unwrap();
    assert!(data.other("minecraft:tool").unwrap().get("future:field").is_some());
    assert!(data.other("minecraft:custom_data").unwrap().get("fixture:unknown").is_some());
}

#[test]
fn decoder_does_not_normalize_single_block_to_wire_list() {
    use horizon_items::{item_data::canon_stack, item_nbt::{item_from_nbt, item_to_nbt}};
    let prefix = r##"{id:"tconstruct:pickaxe",count:1,components:{"minecraft:tool":{rules:[{blocks:"#mineable/pickaxe",speed:2.0f},{blocks:"stone",speed:1.0f}]}}}"##;
    let normalized = r##"{id:"tconstruct:pickaxe",count:1,components:{"minecraft:tool":{rules:[{blocks:"#minecraft:mineable/pickaxe",speed:2.0f},{blocks:["minecraft:stone"],speed:1.0f}],default_mining_speed:1.0f,damage_per_block:1,can_destroy_blocks_in_creative:1b}}}"##;
    let a = parse_stack(prefix).unwrap().unwrap();
    let b = parse_stack(normalized).unwrap().unwrap();
    let a = item_from_nbt(&item_to_nbt(&a)).unwrap();
    let b = item_from_nbt(&item_to_nbt(&b)).unwrap();
    assert_ne!(canon_stack(Some(&a)), canon_stack(Some(&b)));
}

//! Focused test of the real patch guard and repair commit helper.
use horizon_items::{item_data::{parse_stack, Wire}, items::ItemStack};
use horizon_protocol::{buf::{Codec, Reader, Writer}, item::{component, Component, DelimitedSlot, Stack}};
use std::sync::Arc;
use crate::{fixed_set_damage, repair_from_slot, take_repair, tconstruct_source_tool_normalize::{finalize_tool, patch_safe_for_tool_mutation}};

fn damaged_tool(removal: bool) -> ItemStack {
    let input: serde_json::Value = serde_json::from_str(include_str!("cases.json")).unwrap();
    let mut stack = parse_stack(input["cases"][0]["snbt"].as_str().unwrap()).unwrap().unwrap();
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
    let original = damaged_tool(true);
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
    let original = damaged_tool(false);
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

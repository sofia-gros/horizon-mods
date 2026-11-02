//! Run with the fixed Horizon crates and PlutoLib's staged tool codec.
use horizon_items::item_data::parse_stack;
use horizon_world::nbt::Tag;
use serde_json::Value;
use crate::tconstruct_source_tool_normalize::finalize_tool;

#[test]
fn generated_tool_finalizes_once_and_keeps_mirror_and_other_components() {
    let fixture: Value = serde_json::from_str(include_str!("cases.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap().iter().take(2) {
        let mut stack = parse_stack(case["snbt"].as_str().unwrap()).unwrap().unwrap();
        let before = stack.clone();
        finalize_tool(&mut stack, |_| None).unwrap(); // Fixture's Tool rules are tags.
        assert_eq!(stack.item, before.item);
        assert_eq!(stack.count, before.count);
        assert_eq!(stack.damage, before.damage);
        let data = stack.data().unwrap();
        let custom = data.other("minecraft:custom_data").unwrap();
        let tool = data.other("minecraft:tool").unwrap();
        assert!(custom.get("tic_mining_tool") == Some(tool));
        assert!(matches!(data.other("minecraft:max_damage"), Some(Tag::Int(60))));
        assert!(custom.get("tic_materials").is_some());
        let first = stack.clone();
        finalize_tool(&mut stack, |_| None).unwrap();
        assert!(first.same_item_same_components(&stack));
    }
}

#[test]
fn rejected_rule_preserves_the_original_stack() {
    let mut stack = parse_stack(r##"{id:"tconstruct:pickaxe",count:1,components:{"minecraft:tool":{rules:[{blocks:"example:missing",speed:2.0f}]},"minecraft:custom_data":{tic_mining_tool:{rules:[{blocks:"example:missing",speed:2.0f}]}}}}"##).unwrap().unwrap();
    let original = stack.clone();
    assert!(finalize_tool(&mut stack, |_| None).is_err());
    assert!(original.same_item_same_components(&stack));
    assert_eq!(original.count, stack.count);
}

#[test]
fn explicit_block_is_accepted_only_when_registry_resolves_it() {
    let raw = r##"{id:"tconstruct:pickaxe",count:1,components:{"minecraft:tool":{rules:[{blocks:"minecraft:stone",speed:2.0f}]},"minecraft:custom_data":{tic_mining_tool:{rules:[{blocks:"minecraft:stone",speed:2.0f}]}}}}"##;
    let mut stack = parse_stack(raw).unwrap().unwrap();
    finalize_tool(&mut stack, |id| (id == "minecraft:stone").then_some(7)).unwrap();
    let first = stack.clone();
    finalize_tool(&mut stack, |id| (id == "minecraft:stone").then_some(7)).unwrap();
    assert!(first.same_item_same_components(&stack));
}

#[test]
fn saved_mirror_without_tool_is_not_silently_accepted() {
    let raw = r##"{id:"tconstruct:pickaxe",count:1,components:{"minecraft:custom_data":{tic_mining_tool:{rules:[{blocks:"#minecraft:mineable/pickaxe",speed:2.0f}]}}}}"##;
    let mut stack = parse_stack(raw).unwrap().unwrap();
    let before = stack.clone();
    assert!(finalize_tool(&mut stack, |_| None).is_err());
    assert!(before.same_item_same_components(&stack));
}

#[test]
fn cached_patch_is_kept_for_noop_and_not_silently_invalidated() {
    use horizon_items::item_data::Wire;
    use std::sync::Arc;
    let raw = r##"{id:"tconstruct:pickaxe",count:1,components:{"minecraft:tool":{rules:[{blocks:"minecraft:stone",speed:2.0f}]},"minecraft:custom_data":{tic_mining_tool:{rules:[{blocks:"minecraft:stone",speed:2.0f}]}}}}"##;
    let resolve = |id: &str| (id == "minecraft:stone").then_some(7);
    let mut unchanged = parse_stack(raw).unwrap().unwrap();
    finalize_tool(&mut unchanged, resolve).unwrap();
    let snapshot = unchanged.clone();
    unchanged.data_mut().wire = Wire::new(snapshot, Arc::from([1u8, 2, 3]));
    finalize_tool(&mut unchanged, resolve).unwrap();
    assert_eq!(&*unchanged.wire_patch().unwrap(), &[1, 2, 3]);

    let mut changed = parse_stack(raw).unwrap().unwrap();
    let snapshot = changed.clone();
    changed.data_mut().wire = Wire::new(snapshot, Arc::from([1u8, 2, 3]));
    assert!(finalize_tool(&mut changed, resolve).is_err());
    assert_eq!(&*changed.wire_patch().unwrap(), &[1, 2, 3]);
}

#[test]
fn bare_tag_and_block_ids_use_the_minecraft_namespace() {
    let raw = r##"{id:"tconstruct:pickaxe",count:1,components:{"minecraft:tool":{rules:[{blocks:"#mineable/pickaxe",speed:2.0f},{blocks:"stone",speed:1.0f}]},"minecraft:custom_data":{tic_mining_tool:{rules:[{blocks:"#mineable/pickaxe",speed:2.0f},{blocks:"stone",speed:1.0f}]}}}}"##;
    let mut stack = parse_stack(raw).unwrap().unwrap();
    finalize_tool(&mut stack, |id| (id == "minecraft:stone").then_some(7)).unwrap();
    let canonical = horizon_items::item_data::canon_tag(stack.data().unwrap().other("minecraft:tool").unwrap());
    assert!(canonical.contains("#minecraft:mineable/pickaxe"));
    assert!(canonical.contains("minecraft:stone"));
    let before = stack.clone();
    finalize_tool(&mut stack, |id| (id == "minecraft:stone").then_some(7)).unwrap();
    assert!(before.same_item_same_components(&stack));
}

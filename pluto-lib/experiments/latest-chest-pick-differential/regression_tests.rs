// Source-only finite policy comparisons. Native ItemStack/Tag and pick_item are
// real dependencies; the input metadata reader does not model a Game or actor.
use super::{new_policy, old_policy, FixtureMetadata};
use horizon_items::{item_data::{BundleSelected, Wire}, items::ItemStack};
use horizon_world::nbt::{JavaString, Tag};
use std::sync::Arc;

const CHEST: &str = "tconstruct:tinkers_chest";
const KEY: &str = "tconstruct:modifier_chest";
const POS: [i32; 3] = [7, 65, -9];

fn nested(color: Tag) -> Tag {
    Tag::compound([(KEY, Tag::compound([("color", color)]))])
}
fn color(stack: &ItemStack) -> Option<i32> { stack.data().and_then(|d| d.dyed_color) }
fn stack_with_color(value: i32) -> ItemStack {
    let mut stack = ItemStack::new(CHEST, 1);
    stack.data_mut().dyed_color = Some(value);
    stack
}
fn compare(value: Option<ItemStack>, raw: Option<&Tag>) -> Option<ItemStack> {
    let before = raw.cloned();
    let old_meta = FixtureMetadata::new(raw);
    let new_meta = FixtureMetadata::new(raw);
    let old = old_policy::apply(value.clone(), &old_meta, POS);
    // The native ? -> None route never reaches the final owned LOAD.
    let new = value.map(|stack| new_policy::apply(stack, &new_meta, POS));
    assert_eq!(old, new);
    assert_eq!(old_meta.reads.get(), new_meta.reads.get());
    assert_eq!(before.as_ref(), raw);
    new
}

#[test]
fn none_bypasses_both_metadata_and_new_owned_policy() {
    let raw = nested(Tag::Int(0x123456));
    let old_meta = FixtureMetadata::new(Some(&raw));
    let new_meta = FixtureMetadata::new(Some(&raw));
    assert!(old_policy::apply(None, &old_meta, POS).is_none());
    let mut entered = 0;
    let result = None::<ItemStack>.map(|s| { entered += 1; new_policy::apply(s, &new_meta, POS) });
    assert!(result.is_none());
    assert_eq!((entered, old_meta.reads.get(), new_meta.reads.get()), (0, 0, 0));
}

#[test]
fn other_block_items_preserve_every_supplied_value_without_metadata_read() {
    let raw = nested(Tag::Int(5));
    for id in ["minecraft:stone", "minecraft:beehive", "minecraft:decorated_pot", "minecraft:light", "tconstruct:part_chest"] {
        let mut stack = ItemStack::new(id, 3);
        stack.damage = 6;
        stack.data_mut().dyed_color = Some(9);
        stack.data_mut().other.push(("future:field".into(), Tag::IntArray(vec![1, -2])));
        let input = FixtureMetadata::new(Some(&raw));
        let result = new_policy::apply(stack.clone(), &input, POS);
        assert_eq!(result, stack);
        assert_eq!(input.reads.get(), 0);
        assert_eq!(compare(Some(stack.clone()), Some(&raw)), Some(stack));
    }
}

#[test]
fn all_signed_color_boundaries_are_copied_without_rgb_clamp() {
    for value in [i32::MIN, -1, 0, 0x00ff_ffff, i32::MAX] {
        let raw = nested(Tag::Int(value));
        let got = compare(Some(stack_with_color(12)), Some(&raw)).unwrap();
        assert_eq!(color(&got), Some(value));
    }
}

#[test]
fn missing_or_wrong_typed_colors_preserve_existing_color_and_empty_extra() {
    let empty = Tag::compound([] as [(&str, Tag); 0]);
    let wrong = [Tag::Byte(3), Tag::Short(3), Tag::Long(3), Tag::String("3".into()), Tag::End];
    let mut rows = vec![None, Some(empty), Some(Tag::Int(88))];
    rows.extend(wrong.into_iter().map(|x| Some(nested(x))));
    for raw in &rows {
        let got = compare(Some(stack_with_color(17)), raw.as_ref()).unwrap();
        assert_eq!(color(&got), Some(17));
        let got = compare(Some(ItemStack::new(CHEST, 1)), raw.as_ref()).unwrap();
        assert!(got.extra.is_none());
    }
}

#[test]
fn nested_color_wins_over_valid_legacy_color() {
    let raw = Tag::compound([
        ("id", Tag::String(KEY.into())), ("color", Tag::Int(4)),
        (KEY, Tag::compound([("color", Tag::Int(8))])),
    ]);
    assert_eq!(color(&compare(Some(stack_with_color(1)), Some(&raw)).unwrap()), Some(8));
}

#[test]
fn malformed_or_empty_nested_payload_does_not_fall_back_to_legacy() {
    for nested in [Tag::Int(3), Tag::compound([] as [(&str, Tag); 0]), Tag::compound([("color", Tag::Long(8))])] {
        let raw = Tag::compound([("id", Tag::String(KEY.into())), ("color", Tag::Int(4)), (KEY, nested)]);
        assert_eq!(color(&compare(Some(stack_with_color(1)), Some(&raw)).unwrap()), Some(1));
    }
}

#[test]
fn present_nested_payload_preserves_original_loose_id_policy() {
    for id in [Tag::String("future:entity".into()), Tag::Int(3)] {
        let raw = Tag::compound([("id", id), (KEY, Tag::compound([("color", Tag::Int(11))]))]);
        assert_eq!(color(&compare(Some(stack_with_color(1)), Some(&raw)).unwrap()), Some(11));
    }
}

#[test]
fn legacy_fallback_requires_matching_string_id() {
    for (id, expected) in [(Tag::String(KEY.into()), 6), (Tag::String("future:entity".into()), 2), (Tag::Int(4), 2)] {
        let raw = Tag::compound([("id", id), ("color", Tag::Int(6))]);
        assert_eq!(color(&compare(Some(stack_with_color(2)), Some(&raw)).unwrap()), Some(expected));
    }
}

#[test]
fn foreign_utf16_and_nested_metadata_remain_unmodified() {
    let mut fields = horizon_world::nbt::Compound::new();
    fields.insert(JavaString(vec![0xd800]), Tag::List(vec![Tag::Float(0x7fc0_1234), Tag::String(JavaString(vec![0xdc00]))]));
    fields.insert(KEY.into(), Tag::compound([("color", Tag::Int(8)), ("foreign", Tag::Long(i64::MIN))]));
    let raw = Tag::Compound(fields);
    let before = raw.clone();
    assert_eq!(color(&compare(Some(ItemStack::new(CHEST, 1)), Some(&raw)).unwrap()), Some(8));
    assert_eq!(raw, before);
}

#[test]
fn actual_stack_fields_and_runtime_selection_survive_color_only_change() {
    let raw = nested(Tag::Int(8));
    let mut stack = ItemStack::new(CHEST, 7);
    stack.damage = 19;
    let d = stack.data_mut();
    d.custom_name = Some(Tag::compound([("translate", Tag::String("example:chest".into()))]));
    d.container.push((2, ItemStack::new("example:contents", 13)));
    d.bundle_contents.push(ItemStack::new("example:bundle", 4));
    d.bundle_selected = BundleSelected(0);
    d.map_post_processing = Some(1);
    d.other.push(("future:opaque".into(), Tag::String(JavaString(vec![0xd800]))));
    let mut expected = stack.clone(); expected.data_mut().dyed_color = Some(8);
    let got = compare(Some(stack), Some(&raw)).unwrap();
    assert_eq!(got, expected);
    assert_eq!(got.data().unwrap().bundle_selected.0, 0);
    assert_eq!(got.data().unwrap().container, expected.data().unwrap().container);
    assert_eq!(got.data().unwrap().other, expected.data().unwrap().other);
}

#[test]
fn retained_wire_bytes_are_not_reconstructed_but_a_changed_color_stales_use() {
    // Deliberate synthetic policy input, not proof picked_stack creates Wire.
    let mut stack = stack_with_color(2);
    let patch: Arc<[u8]> = Arc::from(vec![0xa5_u8, 0, 0xff]);
    let decoded = stack.clone();
    stack.data_mut().wire = Wire::new(decoded, patch.clone());
    assert_eq!(stack.wire_patch().as_deref(), Some(&*patch));
    let raw = nested(Tag::Int(8));
    let got = compare(Some(stack), Some(&raw)).unwrap();
    assert_eq!(got.data().unwrap().wire.patch(), Some(&*patch));
    assert!(got.wire_patch().is_none());
}

#[test]
fn survival_does_not_grant_or_select_a_different_colored_stack() {
    let raw = nested(Tag::Int(8));
    let picked = compare(Some(ItemStack::new(CHEST, 1)), Some(&raw)).unwrap();
    let mut slots = vec![None; 36]; slots[0] = Some(stack_with_color(3));
    let before = slots.clone(); let mut selected = 2;
    assert!(!crate::native_pick::pick(&mut slots, &mut selected, &picked, false));
    assert_eq!((slots, selected), (before, 2));
}

#[test]
fn survival_selects_the_matching_colored_stack() {
    let raw = nested(Tag::Int(8));
    let picked = compare(Some(ItemStack::new(CHEST, 1)), Some(&raw)).unwrap();
    let mut slots = vec![None; 36]; slots[0] = Some(stack_with_color(3)); slots[6] = Some(stack_with_color(8));
    let before = slots.clone(); let mut selected = 2;
    assert!(crate::native_pick::pick(&mut slots, &mut selected, &picked, false));
    assert_eq!((slots, selected), (before, 6));
}

#[test]
fn creative_clones_collected_payload_with_color_when_no_match_exists() {
    let raw = nested(Tag::Int(8));
    for full_payload in [false, true] {
        // Two supplied pre-collected shapes, not execution of include_data's Game branch.
        let mut input = ItemStack::new(CHEST, 1);
        if full_payload { input.data_mut().other.push(("minecraft:block_entity_data".into(), Tag::compound([("future", Tag::Int(9))]))); }
        let picked = compare(Some(input), Some(&raw)).unwrap();
        let mut slots = vec![None; 36]; let mut selected = 2;
        assert!(crate::native_pick::pick(&mut slots, &mut selected, &picked, true));
        assert_eq!(slots[2].as_ref(), Some(&picked));
        assert_eq!(color(slots[2].as_ref().unwrap()), Some(8));
    }
}

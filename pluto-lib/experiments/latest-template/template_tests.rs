// Unrun latest-official fixture source. native-template.rs must be the exact
// f4dc Template declaration/impl and assembly_components/json_tag region.
pub use horizon_items::item_data;
use horizon_items::{item_data::StackData, items::ItemStack};
use horizon_world::{component_patch::Patch, nbt::{JavaString, Tag}};
use serde_json::Value;
use std::sync::Arc;
include!("native-template.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn source() -> ItemStack {
        let (damage, extra) = item_data::decode_components(Some(&json_tag(&json!({
            "minecraft:damage": 5,
            "minecraft:lore": ["Old"],
            "minecraft:custom_data": {"untouched": 7}
        }))));
        ItemStack { item: "minecraft:wooden_sword".into(), count: 1, damage, extra }
    }

    #[test]
    fn absent_and_empty_patch_remain_distinct_raw_templates() {
        let absent = Template::parse(&json!({"id":"minecraft:stone_sword"})).unwrap();
        let empty = Template::parse(&json!({"id":"minecraft:stone_sword","components":{}})).unwrap();
        assert!(absent.components.is_none());
        assert_eq!(empty.components, Some(Tag::Compound(Default::default())));
        assert!(absent.create().same_item_same_components(&empty.create()));
        let input = source(); let before = input.clone();
        assert_eq!(absent.over(&input, 0).damage, 5);
        assert_eq!(empty.over(&input, 0).damage, 5);
        assert_eq!(input, before);
    }

    #[test]
    fn explicit_zero_is_not_recoverable_from_created_stack() {
        let original = Template::parse(&json!({"id":"minecraft:stone_sword","components":{"minecraft:damage":0}})).unwrap();
        assert_eq!(original.components.as_ref().and_then(|c| c.get("minecraft:damage")), Some(&Tag::Int(0)));
        let created = original.create();
        let reconstructed = Template { item: Some(created.item), count: created.count,
            damage: created.damage, extra: created.extra, components: None };
        let input = source();
        // This negative control is why adding None to every reconstructed
        // production template would be wrong, despite identical create() views.
        assert!(original.create().same_item_same_components(&reconstructed.create()));
        assert_eq!(original.over(&input, 0).damage, 0);
        assert_eq!(reconstructed.over(&input, 0).damage, 5);
    }

    #[test]
    fn explicit_removal_replaces_inherited_value_without_mutating_input() {
        let template = Template::parse(&json!({"id":"minecraft:stone_sword","components":{
            "!minecraft:damage":{},"!minecraft:lore":{},"minecraft:custom_name":"New"
        }})).unwrap();
        let input = source(); let before = input.clone();
        let output = template.over(&input, 0);
        let encoded = item_data::encode_components(&output).unwrap();
        assert_eq!(output.damage, 0);
        assert!(encoded.get("minecraft:damage").is_none());
        assert!(encoded.get("minecraft:lore").is_none());
        assert!(matches!(encoded.get("!minecraft:damage"), Some(Tag::Compound(_))));
        assert!(matches!(encoded.get("!minecraft:lore"), Some(Tag::Compound(_))));
        assert_eq!(encoded.get("minecraft:custom_data"), item_data::encode_components(&input).unwrap().get("minecraft:custom_data"));
        assert_eq!(output.custom_name().as_deref(), Some("New"));
        assert_eq!(input, before);
    }

    #[test]
    fn native_clone_keeps_raw_patch_and_same_over_result() {
        let original = Template::parse(&json!({"id":"minecraft:stone_sword","components":{
            "minecraft:damage":0,"!minecraft:lore":{},"minecraft:custom_data":{"new":2}
        }})).unwrap();
        let cloned = original.clone();
        assert_eq!(cloned.components, original.components);
        let input = source();
        assert_eq!(cloned.over(&input, 0), original.over(&input, 0));
    }

    #[test]
    fn none_is_compatible_only_with_the_old_synthetic_create_oracle() {
        let stack = source(); let expected = stack.clone();
        let template = Template { item: Some(stack.item), count: stack.count,
            damage: stack.damage, extra: stack.extra, components: None };
        let actual = template.create();
        assert_eq!(actual.count, expected.count);
        assert!(actual.same_item_same_components(&expected));
        let empty = Template { item: Some("example:output".into()), count: 1,
            damage: 0, extra: None, components: None };
        assert_eq!(empty.create(), ItemStack::new("example:output", 1));
        // These mirror only the two excluded test factories. No arbitrary
        // parsed template, raw-patch equality or transmute oracle is replaced.
    }

    #[test]
    fn transient_map_postprocess_stays_outside_saved_components() {
        let mut input = source(); input.data_mut().map_post_processing = Some(1);
        let before = input.clone();
        let template = Template::parse(&json!({"id":"minecraft:filled_map"})).unwrap();
        let output = template.over(&input, 0);
        assert_eq!(output.data().unwrap().map_post_processing, Some(1));
        assert!(item_data::encode_components(&output).unwrap().get("minecraft:map_post_processing").is_none());
        assert_eq!(input, before);
    }
}

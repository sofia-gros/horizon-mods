//! Small parser/save-codec check; LAN transport is tested separately.
use horizon_items::{item_data::parse_stack, item_nbt::{item_from_nbt, item_to_nbt}};
use serde_json::Value;

#[test]
fn source_stacks_are_parseable_and_survive_item_nbt_roundtrip() {
    let fixture: Value = serde_json::from_str(include_str!("cases.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let original = parse_stack(case["snbt"].as_str().unwrap()).unwrap().unwrap();
        let restored = item_from_nbt(&item_to_nbt(&original)).unwrap();
        assert_eq!(original.count, restored.count, "{id}");
        assert!(original.same_item_same_components(&restored), "{id}");
        if id == "vanilla_stone_count" { assert_eq!(original.count, 17); continue; }
        let custom = original.data().unwrap().other("minecraft:custom_data").unwrap();
        for key in ["tic_materials", "tic_attack_damage", "tic_mining_tool"] {
            assert!(custom.get(key).is_some(), "{id}: {key}");
        }
        if id == "generated_pickaxe" {
            use horizon_world::nbt::Tag;
            assert!(matches!(original.data().unwrap().other("minecraft:max_damage"), Some(Tag::Int(60))));
            assert!(matches!(custom.get("tic_max_damage"), Some(Tag::Int(60))));
            let tool = original.data().unwrap().other("minecraft:tool").unwrap();
            assert!(matches!(tool.get("damage_per_block"), Some(Tag::Int(1))));
            assert!(matches!(tool.get("default_mining_speed"), Some(Tag::Float(v)) if f32::from_bits(*v) == 1.0));
            let Some(Tag::List(rules)) = tool.get("rules") else { panic!("missing rules") };
            assert_eq!(rules.len(), 2);
            assert!(matches!(rules[0].get("correct_for_drops"), Some(Tag::Byte(0))));
            assert!(matches!(rules[1].get("correct_for_drops"), Some(Tag::Byte(1))));
            assert!(matches!(rules[1].get("speed"), Some(Tag::Float(v)) if f32::from_bits(*v) == 2.0));
            assert!(custom.get("tic_mining_tool") == Some(tool));
        }
        if id == "augmented_pickaxe" {
            assert!(custom.get("tic_upgrades").unwrap().get("tconstruct:haste").is_some());
            assert!(custom.get("tic_overslime").is_some());
            assert!(custom.get("fixture:foreign").unwrap().get("nested").is_some());
            assert!(matches!(custom.get("tic_mining_traits"), Some(horizon_world::nbt::Tag::List(items)) if !items.is_empty()));
            assert!(original.data().unwrap().other("minecraft:lore").is_some());
            assert!(original.data().unwrap().other("minecraft:item_model").is_some());
        }
    }
}

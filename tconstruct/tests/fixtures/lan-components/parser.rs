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

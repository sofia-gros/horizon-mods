#[path = "../../mixins/tconstruct/source-original-crafting-remainders.rs"]
mod source;
#[test]
fn original_container_and_non_container_items() {
    for item in ["water_bucket", "lava_bucket", "milk_bucket"] {
        assert_eq!(source::vanilla_remainder(&format!("minecraft:{item}")), Some("minecraft:bucket"));
    }
    for item in ["dragon_breath", "honey_bottle"] {
        assert_eq!(source::vanilla_remainder(&format!("minecraft:{item}")), Some("minecraft:glass_bottle"));
    }
    for item in ["potion", "mushroom_stew", "flint"] {
        assert_eq!(source::vanilla_remainder(&format!("minecraft:{item}")), None);
    }
}

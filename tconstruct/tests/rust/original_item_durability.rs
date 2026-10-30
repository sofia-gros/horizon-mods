#[path = "../../mixins/tconstruct/source-original-item-durability.rs"]
mod source;

#[test]
fn source_tiers_and_unknown_items() {
    assert_eq!(source::max_damage("minecraft:stone_axe"), Some(131));
    assert_eq!(source::max_damage("minecraft:wooden_pickaxe"), Some(59));
    assert_eq!(source::max_damage("minecraft:diamond_sword"), Some(1561));
    assert_eq!(source::max_damage("minecraft:flint_and_steel"), Some(64));
    assert_eq!(source::max_damage("minecraft:leather_helmet"), Some(55));
    assert_eq!(source::max_damage("minecraft:leather_chestplate"), Some(80));
    assert_eq!(source::max_damage("minecraft:leather_leggings"), Some(75));
    assert_eq!(source::max_damage("minecraft:leather_boots"), Some(65));
    assert_eq!(source::max_damage("minecraft:turtle_helmet"), Some(275));
    assert_eq!(source::max_damage("minecraft:unsupported_example"), None);
}

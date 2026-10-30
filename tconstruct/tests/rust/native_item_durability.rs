#[path = "../../mixins/tconstruct/source-native-durability.rs"]
mod source;

#[test]
fn native_registry_drives_supported_damageable_items() {
    let values = [("minecraft:example".to_owned(), 321), ("minecraft:stone".to_owned(), 0), ("example:tool".to_owned(), 100)].into_iter().collect();
    assert_eq!(source::max_damage("minecraft:example", &values, false), Some(321));
    assert_eq!(source::max_damage("minecraft:example", &values, true), None);
    assert_eq!(source::max_damage("minecraft:stone", &values, false), None);
    assert_eq!(source::max_damage("minecraft:missing", &values, false), None);
    assert_eq!(source::max_damage("example:tool", &values, false), None);
}

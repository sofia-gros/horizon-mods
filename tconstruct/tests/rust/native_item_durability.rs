// Minimal host type shim for compiling the production helper with rustc.
extern crate self as horizon_world;
pub mod nbt {
    pub enum Tag { Int(i32), Byte(i8), String(String) }
}

#[path = "../../mixins/tconstruct/source-native-durability.rs"]
mod source;

use std::collections::HashMap;
use nbt::Tag;

fn registry() -> HashMap<String, u32> {
    [("minecraft:example".to_owned(), 321), ("minecraft:stone".to_owned(), 0), ("example:tool".to_owned(), 100), ("minecraft:overflow".to_owned(), i32::MAX as u32 + 1)].into_iter().collect()
}

#[test]
fn native_registry_drives_supported_damageable_items() {
    let values = registry();
    assert_eq!(source::max_damage("minecraft:example", &values, None), Some(321));
    assert_eq!(source::max_damage("minecraft:stone", &values, None), None);
    assert_eq!(source::max_damage("minecraft:missing", &values, None), None);
    assert_eq!(source::max_damage("example:tool", &values, None), None);
    assert_eq!(source::max_damage("minecraft:overflow", &values, None), None);
}

#[test]
fn positive_integer_override_applies_only_to_supported_registered_items() {
    let values = registry();
    for damage in [1, 123, i32::MAX] {
        let tag = Tag::Int(damage);
        assert_eq!(source::max_damage("minecraft:example", &values, Some(&tag)), Some(damage));
        for item in ["minecraft:stone", "minecraft:missing", "example:tool", "minecraft:overflow"] {
            assert_eq!(source::max_damage(item, &values, Some(&tag)), None);
        }
    }
}

#[test]
fn malformed_or_nonpositive_override_does_not_fall_back_to_registry() {
    let values = registry();
    for tag in [Tag::Int(0), Tag::Int(-1), Tag::Byte(1), Tag::String("123".into())] {
        assert_eq!(source::max_damage("minecraft:example", &values, Some(&tag)), None);
    }
}

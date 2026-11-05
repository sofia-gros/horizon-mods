pub(super) fn supported_value(value: &serde_json::Value) -> bool {
    value == &serde_json::json!({"level_display":"tconstruct:default","modules":[{
        "type":"tconstruct:protection","attacker":{"type":"mantle:mob_type","mobs":"minecraft:undead"},
        "damage_source":"mantle:can_protect","each_level":1.25
    }]})
}

pub(super) fn points(levels: impl IntoIterator<Item = i32>) -> Option<f32> {
    let mut total = 0.0f32;
    for level in levels {
        if level < 0 { return None; }
        total += 1.25f32 * level as f32;
    }
    total.is_finite().then_some(total)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn intact_levels_add_before_the_shared_cap() {
        assert_eq!(points([]),Some(0.0));
        assert_eq!(points([1,2,0]),Some(3.75));
        assert_eq!(points([-1]),None);
    }
    #[test] fn only_the_canonical_undead_condition_is_supported() {
        let mut value = serde_json::json!({"level_display":"tconstruct:default","modules":[{
            "type":"tconstruct:protection","attacker":{"type":"mantle:mob_type","mobs":"minecraft:undead"},
            "damage_source":"mantle:can_protect","each_level":1.25
        }]});
        assert!(supported_value(&value));
        value["modules"][0]["attacker"]["mobs"] = serde_json::json!("minecraft:arthropod");
        assert!(!supported_value(&value));
    }
}

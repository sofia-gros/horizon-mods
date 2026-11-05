pub(super) fn supported_value(value: &serde_json::Value) -> bool {
    value == &serde_json::json!({"level_display":"tconstruct:default","modules":[
        {"type":"tconstruct:enchantment_ignoring_protection","level":1,"name":"minecraft:fire_protection"},
        {"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":[
            "mantle:can_protect",{"type":"mantle:attacker","entity_type":"mantle:fire_immune","which":"causing"}
        ]},"each_level":1.25}
    ]})
}

/// Both modules generate the same enchantment, so their integer levels add on
/// one piece before the highest equipped Fire Protection level is selected.
pub(super) fn combined_enchantment(fire: i32, scorch: i32) -> Option<i32> {
    if fire < 0 || scorch < 0 { return None; }
    fire.checked_add(scorch)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fire_and_scorch_add_on_the_same_piece() {
        assert_eq!(combined_enchantment(2,3),Some(5));
        assert_eq!(combined_enchantment(0,2),Some(2));
        assert_eq!(combined_enchantment(-1,2),None);
        assert_eq!(combined_enchantment(i32::MAX,1),None);
    }
    #[test]
    fn the_canonical_enchantment_and_attacker_condition_are_required() {
        let mut value = serde_json::json!({"level_display":"tconstruct:default","modules":[
            {"type":"tconstruct:enchantment_ignoring_protection","level":1,"name":"minecraft:fire_protection"},
            {"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":[
                "mantle:can_protect",{"type":"mantle:attacker","entity_type":"mantle:fire_immune","which":"causing"}
            ]},"each_level":1.25}
        ]});
        assert!(supported_value(&value));
        value["modules"][1]["damage_source"]["predicates"][1]["which"] = serde_json::json!("direct");
        assert!(!supported_value(&value));
    }
}


/// Explicit absence wins over stale projectile state. Only the projectile
/// path may use its recorded owner; ordinary melee has no verified identity.
pub(super) fn causing_id(known: Option<Option<u32>>, projectile: bool, owner: Option<u32>) -> Option<u32> {
    match known { Some(cause) => cause, None if projectile => owner, None => None }
}

#[cfg(test)] mod causing_tests {
    use super::*;
    #[test] fn known_identity_and_projectile_owner_are_not_position_guesses() {
        assert_eq!(causing_id(Some(Some(7)),false,Some(8)),Some(7));
        assert_eq!(causing_id(Some(None),true,Some(8)),None);
        assert_eq!(causing_id(None,true,Some(8)),Some(8));
        assert_eq!(causing_id(None,false,Some(8)),None);
        assert_eq!(causing_id(None,true,None),None);
    }
}

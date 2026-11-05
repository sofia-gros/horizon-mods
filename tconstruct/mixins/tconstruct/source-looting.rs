// Bounded source Looting modules. No experience multiplier is projected.
const LOOTING: &str = r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:weapon_looting","level":1,"tool":{"type":"mantle:or","predicates":[{"type":"mantle:set","items":["minecraft:air"]},{"type":"mantle:tag","tag":"tconstruct:modifiable/melee"}]}},{"type":"tconstruct:armor_looting","level":1,"slots":["feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor/worn"}}]}"#;
const LOOTER: &str = r#"{"level_display":{"type":"tconstruct:unique","first_unique":false,"unique_until":2},"modules":[{"type":"tconstruct:armor_looting","level":{"flat":1},"slots":["feet","legs","chest","head"]},{"type":"tconstruct:attribute","attribute":"tconstruct:player.experience_multiplier","flat":0.25,"modifier_level":{"min":2},"operation":"multiply_base","slots":["mainhand","offhand","feet","legs","chest","head"]}]}"#;
const FORTUNATE: &str = r#"{"level_display":{"type":"tconstruct:unique","first_unique":false,"unique_until":2},"modules":[{"type":"tconstruct:constant_enchantment","level":{"flat":1},"name":"minecraft:fortune","tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/harvest"}},{"type":"tconstruct:attribute","attribute":"minecraft:generic.luck","flat":1.0,"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}},{"type":"tconstruct:armor_looting","level":{"flat":1},"modifier_level":{"min":2},"slots":["feet","legs","chest","head"]}]}"#;

pub(crate) fn known(id: &str, bytes: Option<&[u8]>) -> bool {
    let expected = match id {
        "tconstruct:looting" => LOOTING,
        "tconstruct:looter" => LOOTER,
        "tconstruct:fortunate" => FORTUNATE,
        _ => return false,
    };
    bytes
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(expected).ok())
}

pub(crate) fn armor_bonus(looting: i32, looter: i32, fortunate: i32) -> Option<i32> {
    if looting < 0 || looter < 0 || fortunate < 0 {
        return None;
    }
    looting
        .checked_add(i32::from(looter > 0))?
        .checked_add(i32::from(fortunate >= 2))
}

pub(crate) fn total(base: i32, bonuses: &[i32]) -> Option<i32> {
    if base < 0 {
        return None;
    }
    let value = bonuses.iter().try_fold(base, |sum, bonus| {
        if *bonus < 0 {
            None
        } else {
            sum.checked_add(*bonus)
        }
    })?;
    (value <= 255).then_some(value)
}

// Source projectile DummyToolStack starts from zero rather than the current
// weapon's native Looting. Luck's separate module remains outside this capture.
pub(crate) fn arrow_level(looting: i32, luck: i32) -> Option<i32> {
    (looting > 0 && looting <= 255 && luck == 0).then_some(looting)
}
pub(crate) fn base(native: i32, projectile: Option<i32>) -> Option<i32> {
    if native < 0 {
        return None;
    }
    match projectile {
        Some(level) => arrow_level(level, 0).map(|_| 0),
        None => (native >= 0).then_some(native),
    }
}

pub(crate) fn armor_slot(item: &str) -> Option<usize> {
    Some(match item {
        "tconstruct:travelers_helmet" | "tconstruct:plate_helmet" | "tconstruct:slime_helmet" => 0,
        "tconstruct:travelers_chestplate"
        | "tconstruct:plate_chestplate"
        | "tconstruct:slimy_chestplate"
        | "tconstruct:slime_wings" => 1,
        "tconstruct:travelers_leggings"
        | "tconstruct:plate_leggings"
        | "tconstruct:slime_leggings" => 2,
        "tconstruct:travelers_boots" | "tconstruct:plate_boots" | "tconstruct:slime_boots" => 3,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projectile_base_does_not_inherit_the_current_weapon() {
        assert_eq!(base(8, Some(2)), Some(0));
        assert_eq!(total(base(8, Some(2)).unwrap(), &[2, 1, 1]), Some(4));
        assert_eq!(base(8, None), Some(8));
        assert_eq!(base(8, Some(0)), None);
        assert_eq!(arrow_level(2, 1), None);
        assert_eq!(arrow_level(256, 0), None);
    }
    #[test]
    fn armor_sources_have_distinct_level_rules() {
        assert_eq!(armor_bonus(3, 0, 0), Some(3));
        assert_eq!(armor_bonus(0, 5, 1), Some(1));
        assert_eq!(armor_bonus(0, 5, 2), Some(2));
        assert_eq!(armor_bonus(3, 1, 9), Some(5));
        assert_eq!(armor_bonus(-1, 1, 2), None);
        assert_eq!(armor_bonus(i32::MAX, 1, 2), None);
    }
    #[test]
    fn equipment_contributions_add_without_overflow() {
        assert_eq!(total(2, &[3, 1, 1, 0, 2]), Some(9));
        assert_eq!(total(0, &[]), Some(0));
        assert_eq!(total(255, &[1]), None);
        assert_eq!(total(-1, &[1]), None);
        assert_eq!(total(1, &[-1]), None);
    }
    #[test]
    fn definitions_and_slots_are_bounded() {
        for (id, definition) in [
            ("tconstruct:looting", LOOTING),
            ("tconstruct:looter", LOOTER),
            ("tconstruct:fortunate", FORTUNATE),
        ] {
            assert!(known(id, Some(definition.as_bytes())));
            assert!(!known(id, None));
        }
        let mut changed: serde_json::Value = serde_json::from_str(LOOTING).unwrap();
        changed["modules"][0]["level"] = serde_json::json!(2);
        assert!(!known(
            "tconstruct:looting",
            Some(&serde_json::to_vec(&changed).unwrap())
        ));
        assert_eq!(armor_slot("tconstruct:slime_leggings"), Some(2));
        assert_eq!(armor_slot("minecraft:iron_leggings"), None);
    }
}

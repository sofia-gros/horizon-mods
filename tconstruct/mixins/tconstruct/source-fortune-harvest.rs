// Source enchantment projection for the bounded player-harvest path.
const FORTUNE: &str = r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:constant_enchantment","level":1,"name":"minecraft:fortune","tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/harvest"}},{"type":"tconstruct:armor_harvest_enchantment","level":1,"name":"minecraft:fortune","slots":["feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor/worn"}},{"type":"tconstruct:stat_boost","each_level":1.0,"operation":"add","stat":"tconstruct:sea_luck"},{"type":"tconstruct:attribute","attribute":"minecraft:generic.luck","each_level":1.0,"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor/worn"}}]}"#;
const FORTUNATE: &str = r#"{"level_display":{"type":"tconstruct:unique","first_unique":false,"unique_until":2},"modules":[{"type":"tconstruct:constant_enchantment","level":{"flat":1},"name":"minecraft:fortune","tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/harvest"}},{"type":"tconstruct:attribute","attribute":"minecraft:generic.luck","flat":1.0,"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}},{"type":"tconstruct:armor_looting","level":{"flat":1},"modifier_level":{"min":2},"slots":["feet","legs","chest","head"]}]}"#;

pub(super) fn known(id: &str, bytes: Option<&[u8]>) -> bool {
    let expected = match id {
        "tconstruct:fortune" => FORTUNE,
        "tconstruct:fortunate" => FORTUNATE,
        _ => return false,
    };
    bytes
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(expected).ok())
}

// Fortune uses LevelingInt.LEVEL, Fortunate uses flat=1 regardless of its
// positive modifier level. Preserve native enchantments and fail closed outside
// the ordinary enchantment range rather than wrapping or truncating.
pub(super) fn level(base: u32, fortune: i32, fortunate: i32) -> Option<u32> {
    if fortune < 0 || fortunate < 0 {
        return None;
    }
    let result = base
        .checked_add(fortune as u32)?
        .checked_add(u32::from(fortunate > 0))?;
    (result <= 255).then_some(result)
}

pub(super) fn with_worn(base: u32, levels: &[i32]) -> Option<u32> {
    let result = levels.iter().try_fold(base, |total, level| {
        if *level < 0 {
            None
        } else {
            total.checked_add(*level as u32)
        }
    })?;
    (result <= 255).then_some(result)
}

pub(super) fn armor_slot(item: &str) -> Option<usize> {
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
    fn worn_levels_add_once_to_the_tool_projection() {
        assert_eq!(with_worn(level(1, 2, 8).unwrap(), &[1, 0, 2, 3]), Some(10));
        assert_eq!(with_worn(0, &[1, 1, 0, 0]), Some(2));
        assert_eq!(with_worn(255, &[1]), None);
        assert_eq!(with_worn(0, &[-1]), None);
        assert_eq!(with_worn(u32::MAX, &[1]), None);
    }
    #[test]
    fn only_stock_worn_slots_are_projected() {
        assert_eq!(armor_slot("tconstruct:slime_helmet"), Some(0));
        assert_eq!(armor_slot("tconstruct:slime_wings"), Some(1));
        assert_eq!(armor_slot("tconstruct:plate_leggings"), Some(2));
        assert_eq!(armor_slot("tconstruct:travelers_boots"), Some(3));
        assert_eq!(armor_slot("tconstruct:travelers_shield"), None);
    }
    #[test]
    fn fortune_scales_but_fortunate_is_flat() {
        assert_eq!(level(0, 3, 0), Some(3));
        assert_eq!(level(2, 3, 9), Some(6));
        assert_eq!(level(0, 0, 1), Some(1));
        assert_eq!(level(0, 0, 0), Some(0));
        assert_eq!(level(0, -1, 1), None);
        assert_eq!(level(0, 1, -1), None);
        assert_eq!(level(255, 1, 0), None);
        assert_eq!(level(u32::MAX, 1, 1), None);
    }
    #[test]
    fn altered_or_missing_definitions_do_not_gain_source_behavior() {
        assert!(known("tconstruct:fortune", Some(FORTUNE.as_bytes())));
        assert!(known("tconstruct:fortunate", Some(FORTUNATE.as_bytes())));
        assert!(!known("tconstruct:fortune", Some(FORTUNATE.as_bytes())));
        assert!(!known("tconstruct:fortune", None));
        let mut changed: serde_json::Value = serde_json::from_str(FORTUNE).unwrap();
        changed["modules"][0]["level"] = serde_json::json!(2);
        assert!(!known(
            "tconstruct:fortune",
            Some(&serde_json::to_vec(&changed).unwrap())
        ));
    }
}

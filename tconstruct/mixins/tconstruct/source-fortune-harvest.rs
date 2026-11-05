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

#[cfg(test)]
mod tests {
    use super::*;
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

// Magic Protection's secondary attribute, not its damage-protection module.
pub(super) const ID: &str = "tconstruct:magic_protection";
const DEFINITION: &str = r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:max_armor_attribute","allow_broken":false,"attribute":"tconstruct:generic.bad_effect_duration_multiplier","each_level":-0.05,"held_tag":"tconstruct:modifiable/held","operation":"multiply_base"},{"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":["mantle:can_protect",{"type":"mantle:tag","tag":"tconstruct:protection/magic"}]},"each_level":3.0}]}"#;

pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes.and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(DEFINITION).ok())
}

/// Five reinforcement units per level. Source IncrementalModifierEntry first
/// adds the started level, then applies its negative incomplete-level fraction.
pub(super) fn effective_level(trait_level: i32, raw: i32) -> Option<f32> {
    if trait_level < 0 || !(0..=5 * 32767).contains(&raw) { return None; }
    let whole = trait_level.checked_add(raw / 5)?;
    let remainder = raw % 5;
    Some(if remainder == 0 { whole as f32 }
        else { whole.checked_add(1)? as f32 + (remainder - 5) as f32 / 5.0 })
}

pub(super) fn amount(levels: &[f32]) -> Option<f64> {
    if levels.iter().any(|level| !level.is_finite() || *level < 0.0) { return None; }
    let maximum = levels.iter().copied().fold(0.0f32, f32::max);
    Some(f64::from(0.0f32 + -0.05f32 * maximum))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_keeps_source_float_order_and_trait_level_once() {
        assert_eq!(effective_level(0, 1), Some(1.0 + -4.0 / 5.0));
        assert_eq!(effective_level(2, 7), Some(4.0 + -3.0 / 5.0));
        assert_eq!(effective_level(2, 5), Some(3.0));
        assert_eq!(effective_level(0, 5 * 32767), Some(32767.0));
        assert_eq!(effective_level(-1, 0), None);
        assert_eq!(effective_level(0, -1), None);
        assert_eq!(effective_level(0, 5 * 32767 + 1), None);
        assert_eq!(effective_level(i32::MAX, 1), None);
    }
    #[test]
    fn strongest_piece_wins_without_summing_armor_and_hands() {
        let one = f64::from(-0.05f32);
        assert_eq!(amount(&[1.0, 1.0, 0.4]), Some(one));
        assert_eq!(amount(&[1.0, 2.0, 3.0, 0.0, 2.0, 3.0]), Some(f64::from(-0.05f32 * 3.0)));
        assert_eq!(amount(&[]), Some(0.0));
        assert_eq!(amount(&[f32::NAN]), None);
        assert_eq!(amount(&[-1.0]), None);
    }
    #[test]
    fn definition_override_must_preserve_complete_stock_modules() {
        assert!(known(Some(DEFINITION.as_bytes())));
        assert!(!known(None));
        let mut value: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        value["modules"][0]["allow_broken"] = serde_json::json!(true);
        assert!(!known(Some(&serde_json::to_vec(&value).unwrap())));
    }
}

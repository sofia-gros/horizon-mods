// Ricochet is an ordinary AttributeModule (not percent EquipmentChange hooks).
pub(super) const ID: &str = "tconstruct:ricochet";
const DEFINITION: &str = r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:attribute","attribute":"tconstruct:generic.knockback_multiplier","each_level":0.2,"operation":"multiply_base","slots":["mainhand","offhand","feet","legs","chest","head"]}]}"#;

pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(DEFINITION).ok())
}

pub(super) fn multiplier(levels: &[i32]) -> Option<f64> {
    levels
        .iter()
        .try_fold(1.0, |value, level| {
            (*level >= 0).then_some(value + f64::from(0.0f32 + 0.2f32 * *level as f32))
        })
        .map(|value| value.clamp(0.0, 100.0))
}

// ModifierEvents explicitly casts the scaled event strength to float, before
// vanilla applies knockback resistance. No active multiplier leaves it alone.
pub(super) fn strength(power: f64, factor: f64) -> f64 {
    if factor == 1.0 || !power.is_finite() || !factor.is_finite() || factor < 0.0 {
        return power;
    }
    f64::from((power * factor) as f32)
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
pub(super) fn held_shield(item: &str) -> bool {
    matches!(
        item,
        "tconstruct:travelers_shield" | "tconstruct:plate_shield" | "tconstruct:battlesign"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn base_multipliers_add_instead_of_multiplying_per_piece() {
        let one = f64::from(0.2f32);
        assert_eq!(multiplier(&[1, 1]), Some(1.0 + one + one));
        assert_ne!(multiplier(&[1, 1]), Some((1.0 + one) * (1.0 + one)));
        assert_eq!(multiplier(&[]), Some(1.0));
        assert_eq!(multiplier(&[-1]), None);
        assert_eq!(multiplier(&[i32::MAX]), Some(100.0));
    }
    #[test]
    fn strength_uses_source_float_rounding_only_when_changed() {
        assert_eq!(strength(0.4, 1.0), 0.4);
        let factor = multiplier(&[1]).unwrap();
        assert_eq!(strength(0.4, factor), f64::from((0.4 * factor) as f32));
        assert_ne!(strength(0.4, factor), 0.4 * factor);
    }
    #[test]
    fn source_gate_and_slot_scope_are_explicit() {
        assert!(known(Some(DEFINITION.as_bytes())));
        let mut definition: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        definition["modules"][0]["operation"] = serde_json::json!("multiply_total");
        assert!(!known(Some(&serde_json::to_vec(&definition).unwrap())));
        assert!(!known(None));
        assert_eq!(armor_slot("tconstruct:plate_helmet"), Some(0));
        assert!(held_shield("tconstruct:plate_shield"));
        assert!(!held_shield("minecraft:shield"));
    }
}

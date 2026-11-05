// Source Balm of Ssssss uses MULTIPLY_BASE, before Crystalstrike's MULTIPLY_TOTAL.
pub(super) const ID: &str = "tconstruct:balm_of_sssss";
const DEFINITION: &str = r#"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:attribute","attribute":"tconstruct:generic.bad_effect_duration_multiplier","each_level":-0.1,"flat":-0.1,"operation":"multiply_base","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"}]}"#;
pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(DEFINITION).ok())
}
pub(super) fn amount(level: i32) -> Option<f64> {
    if level < 0 {
        return None;
    }
    Some(if level == 0 {
        0.0
    } else {
        f64::from(-0.1_f32 + -0.1_f32 * level as f32)
    })
}
/// A base of one, additive base-multiplier contributions, then each separate
/// total multiplier. Clamp the completed attribute, not individual modules.
pub(super) fn multiplier(levels: &[i32], total_amounts: &[f64]) -> Option<f64> {
    multiplier_with_base(levels, &[], total_amounts)
}

/// Other MULTIPLY_BASE modules share the same base of one. Do not round a
/// duration or clamp a partial attribute before all operations are combined.
pub(super) fn multiplier_with_base(
    levels: &[i32],
    base_amounts: &[f64],
    total_amounts: &[f64],
) -> Option<f64> {
    let base = levels
        .iter()
        .try_fold(1.0, |value, level| Some(value + amount(*level)?))?;
    let base = base_amounts.iter().try_fold(base, |value, amount| {
        amount.is_finite().then_some(value + amount)
    })?;
    total_amounts
        .iter()
        .try_fold(base, |value, amount| {
            (amount.is_finite() && *amount >= 0.0).then_some(value * (1.0 + amount))
        })
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 100.0))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flat_terms_are_per_positive_piece_and_base_modifiers_add() {
        let one = f64::from(-0.1_f32 + -0.1_f32);
        assert_eq!(amount(0), Some(0.0));
        assert_eq!(multiplier(&[1, 0, 1], &[]), Some(1.0 + one + one));
        assert_ne!(multiplier(&[1, 1], &[]), multiplier(&[2], &[]));
        assert_eq!(multiplier(&[i32::MAX], &[]), Some(0.0));
        assert_eq!(amount(-1), None);
    }
    #[test]
    fn total_modifiers_apply_after_balm_before_one_duration_rounding() {
        let base = 1.0 + amount(1).unwrap();
        let crystal = f64::from(0.05_f32);
        assert_eq!(
            multiplier(&[1], &[crystal, crystal]),
            Some(base * (1.0 + crystal) * (1.0 + crystal))
        );
        assert_eq!(multiplier(&[], &[1000.0]), Some(100.0));
        assert_eq!(multiplier(&[1], &[f64::NAN]), None);
    }
    #[test]
    fn source_gate_checks_the_operation_and_flat_term() {
        assert!(known(Some(DEFINITION.as_bytes())));
        let mut value: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        value["modules"][0]["operation"] = serde_json::json!("multiply_total");
        assert!(!known(Some(&serde_json::to_vec(&value).unwrap())));
        assert!(!known(None));
    }
}

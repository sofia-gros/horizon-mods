// Source Skyfall's percent-style gravity and ordinary safe-fall attributes.
pub(super) const ID: &str = "tconstruct:skyfall";
const DEFINITION: &str = r#"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:attribute","attribute":"forge:entity_gravity","each_level":-0.1,"flat":-0.05,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"},{"type":"tconstruct:attribute","attribute":"tconstruct:generic.safe_fall_distance","each_level":1.0,"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"]}]}"#;

pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(DEFINITION).ok())
}

// FloatLevelingValue evaluates flat + each_level * level as a float before
// widening to the attribute double. Keep one MULTIPLY_TOTAL modifier per slot.
pub(super) fn gravity(level: i32) -> Option<f64> {
    if level < 0 {
        return None;
    }
    Some(if level == 0 {
        0.0
    } else {
        f64::from(-0.05_f32 + -0.1_f32 * level as f32)
    })
}

// Dragonfall and Skyfall feed the same source attribute with range [-10,100].
// Its native projection is a distance bonus above the native base threshold.
// Clamp the combined contribution once, not each modifier independently.
pub(super) fn combined_safe_fall(dragonfall: f64, levels: &[i32]) -> Option<f64> {
    if !dragonfall.is_finite() || dragonfall < 0.0 {
        return None;
    }
    levels
        .iter()
        .try_fold(dragonfall, |total, level| {
            if *level < 0 {
                None
            } else {
                Some(total + f64::from(1.0_f32 * *level as f32))
            }
        })
        .map(|value| value.clamp(-10.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gravity_is_float_evaluated_then_multiplied_per_slot() {
        assert_eq!(gravity(0), Some(0.0));
        assert_eq!(gravity(1), Some(f64::from(-0.15_f32)));
        assert_eq!(gravity(2), Some(f64::from(-0.25_f32)));
        assert_eq!(gravity(-1), None);
        let one = gravity(1).unwrap();
        assert_eq!(
            (1.0 + one) * (1.0 + one),
            (1.0 + f64::from(-0.15_f32)).powi(2)
        );
        assert_ne!((1.0 + one) * (1.0 + one), 1.0 + one + one);
    }
    #[test]
    fn shared_fall_attribute_caps_after_both_traits() {
        assert_eq!(combined_safe_fall(4.0, &[1, 2, 0, 1]), Some(8.0));
        assert_eq!(combined_safe_fall(99.0, &[2, 2]), Some(100.0));
        assert_eq!(combined_safe_fall(0.0, &[i32::MAX]), Some(100.0));
        assert_eq!(combined_safe_fall(4.0, &[-1]), None);
    }
    #[test]
    fn changed_source_definition_does_not_inherit_skyfall() {
        assert!(known(Some(DEFINITION.as_bytes())));
        let mut value: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        value["modules"][0]["operation"] = "addition".into();
        assert!(!known(Some(&serde_json::to_vec(&value).unwrap())));
        assert!(!known(None));
    }
}

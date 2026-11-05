// Source Godspeed's three percent-style equipment attributes.
pub(super) const ID: &str = "tconstruct:godspeed";
const DEFINITION: &str = r#"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:attribute","attribute":"minecraft:generic.attack_speed","each_level":0.015,"flat":0.01,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"},{"type":"tconstruct:attribute","attribute":"minecraft:generic.movement_speed","each_level":0.025,"flat":0.025,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"},{"type":"tconstruct:attribute","attribute":"tconstruct:player.mining_speed_multiplier","each_level":0.05,"flat":0.025,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"}]}"#;

pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(DEFINITION).ok())
}

#[derive(Clone, Copy)]
pub(super) enum Attribute {
    Attack,
    Movement,
    Mining,
}

pub(super) fn amount(level: i32, attribute: Attribute) -> Option<f64> {
    if level < 0 {
        return None;
    }
    // There is no module invocation at level zero, so no flat bonus either.
    if level == 0 {
        return Some(0.0);
    }
    let (flat, each): (f32, f32) = match attribute {
        Attribute::Attack => (0.01, 0.015),
        Attribute::Movement => (0.025, 0.025),
        Attribute::Mining => (0.025, 0.05),
    };
    Some(f64::from(flat + each * level as f32))
}

pub(super) fn scale(base: f64, levels: &[i32], attribute: Attribute, maximum: f64) -> Option<f64> {
    if !base.is_finite() || base < 0.0 {
        return None;
    }
    levels
        .iter()
        .try_fold(base, |value, level| {
            Some(value * (1.0 + amount(*level, attribute)?))
        })
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, maximum))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_flat_plus_level_coefficients_and_empty_slots() {
        assert_eq!(amount(0, Attribute::Attack), Some(0.0));
        assert_eq!(
            amount(1, Attribute::Attack),
            Some(f64::from(0.01_f32 + 0.015_f32))
        );
        assert_eq!(
            amount(2, Attribute::Attack),
            Some(f64::from(0.01_f32 + 0.015_f32 * 2.0))
        );
        assert_eq!(
            amount(1, Attribute::Movement),
            Some(f64::from(0.025_f32 + 0.025_f32))
        );
        assert_eq!(
            amount(1, Attribute::Mining),
            Some(f64::from(0.025_f32 + 0.05_f32))
        );
        let one = amount(1, Attribute::Attack).unwrap();
        assert_eq!(
            scale(4.0, &[1, 0, 1], Attribute::Attack, 1024.0),
            Some(4.0 * (1.0 + one) * (1.0 + one))
        );
        assert_ne!(
            scale(4.0, &[1, 1], Attribute::Attack, 1024.0),
            scale(4.0, &[2], Attribute::Attack, 1024.0)
        );
        assert_eq!(scale(100.0, &[1], Attribute::Mining, 100.0), Some(100.0));
        assert_eq!(amount(-1, Attribute::Attack), None);
    }
    #[test]
    fn only_the_known_complete_source_definition_is_enabled() {
        assert!(known(Some(DEFINITION.as_bytes())));
        let mut value: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        value["modules"][0]["flat"] = serde_json::json!(0.02);
        assert!(!known(Some(&serde_json::to_vec(&value).unwrap())));
        assert!(!known(None));
        assert!(!known(Some(b"{}")));
    }
}

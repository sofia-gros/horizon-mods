// Pinned Cobalamin equipment attributes. Use-item speed is handled separately.
pub(super) const ID: &str = "tconstruct:cobalamin";
const DEFINITION: &str = r#"{
  "level_display": "tconstruct:single_level",
  "modules": [
    {
      "type": "tconstruct:attribute",
      "attribute": "minecraft:generic.attack_speed",
      "each_level": 0.05,
      "operation": "multiply_total",
      "slots": [
        "mainhand",
        "offhand",
        "feet",
        "legs",
        "chest",
        "head"
      ],
      "tooltip_style": "percent"
    },
    {
      "type": "tconstruct:attribute",
      "attribute": "tconstruct:player.use_item_speed",
      "each_level": 0.1,
      "operation": "addition",
      "slots": [
        "mainhand",
        "offhand",
        "feet",
        "legs",
        "chest",
        "head"
      ],
      "tooltip_style": "percent"
    },
    {
      "type": "tconstruct:attribute",
      "attribute": "tconstruct:player.mining_speed_multiplier",
      "each_level": 0.15,
      "operation": "multiply_total",
      "slots": [
        "mainhand",
        "offhand",
        "feet",
        "legs",
        "chest",
        "head"
      ],
      "tooltip_style": "percent"
    }
  ]
}"#;

pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(DEFINITION).ok())
}

pub(super) fn amount(level: i32, each_level: f32) -> Option<f64> {
    (level >= 0).then(|| f64::from(0.0_f32 + each_level * level as f32))
}

/// Each occupied equipment slot supplies its own MULTIPLY_TOTAL modifier.
pub(super) fn scale(base: f64, levels: &[i32], each_level: f32, maximum: f64) -> Option<f64> {
    if !base.is_finite() || base < 0.0 {
        return None;
    }
    levels
        .iter()
        .try_fold(base, |value, level| {
            Some(value * (1.0 + amount(*level, each_level)?))
        })
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, maximum))
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
    fn source_definition_is_structural_and_not_a_best_guess() {
        assert!(known(Some(DEFINITION.as_bytes())));
        let mut changed: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        changed["modules"][0]["each_level"] = serde_json::json!(0.06);
        assert!(!known(Some(&serde_json::to_vec(&changed).unwrap())));
        assert!(!known(None));
        assert!(!known(Some(b"{}")));
    }
    #[test]
    fn slot_products_keep_source_float_coefficients_and_attribute_caps() {
        let one = f64::from(0.05_f32);
        assert_eq!(
            scale(4.0, &[1, 1], 0.05, 1024.0),
            Some(4.0 * (1.0 + one) * (1.0 + one))
        );
        assert_ne!(
            scale(4.0, &[1, 1], 0.05, 1024.0),
            scale(4.0, &[2], 0.05, 1024.0)
        );
        assert_eq!(
            scale(1.0, &[1], 0.15, 100.0),
            Some(1.0 + f64::from(0.15_f32))
        );
        assert_eq!(scale(100.0, &[1], 0.15, 100.0), Some(100.0));
        assert_eq!(scale(1.0, &[-1], 0.15, 100.0), None);
        assert_eq!(scale(f64::NAN, &[1], 0.15, 100.0), None);
        assert_eq!(armor_slot("tconstruct:slime_chestplate"), None);
        assert_eq!(armor_slot("tconstruct:slimy_chestplate"), Some(1));
    }
}

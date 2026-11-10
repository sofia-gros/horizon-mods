// Source EffectImmunityModule + ModifierEvents.isPotionApplicable.
pub(super) const ID: &str = "tconstruct:shell_gut";
const DEFINITION: &str = r#"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:effect_immunity","effect":"minecraft:poison","max_level":{"each_level":1}},{"type":"tconstruct:effect_immunity","effect":"minecraft:hunger","max_level":{"each_level":1,"flat":1}},{"type":"tconstruct:effect_immunity","effect":"minecraft:nausea","max_level":{"each_level":1}}]}"#;

pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(DEFINITION).ok())
}

/// Multiset counts sum across contributing slots. Hunger has one additional
/// count per positive-level piece, not once for the whole equipment set.
pub(super) fn threshold(effect: &str, levels: &[i32]) -> Option<i32> {
    let hunger = match effect.strip_prefix("minecraft:").unwrap_or(effect) {
        "poison" | "nausea" => false,
        "hunger" => true,
        _ => return None,
    };
    levels.iter().try_fold(0_i32, |sum, level| {
        if *level < 0 {
            return None;
        }
        let contribution = if *level == 0 {
            0
        } else {
            level.checked_add(if hunger { 1 } else { 0 })?
        };
        sum.checked_add(contribution)
    })
}

pub(super) fn immune(effect: &str, amplifier: i32, levels: &[i32]) -> bool {
    // Invalid native input stays on its original path rather than granting an
    // immunity for zero points. Normal effect amplifiers are nonnegative.
    amplifier >= 0 && threshold(effect, levels).is_some_and(|points| points > amplifier)
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
    fn level_one_blocks_poison_one_and_hunger_two_not_the_next_level() {
        for effect in ["poison", "minecraft:nausea"] {
            assert!(immune(effect, 0, &[1]));
            assert!(!immune(effect, 1, &[1]));
        }
        assert!(immune("minecraft:hunger", 1, &[1]));
        assert!(!immune("hunger", 2, &[1]));
        assert!(!immune("poison", 0, &[0]));
        assert!(!immune("poison", -1, &[0]));
    }
    #[test]
    fn pieces_add_immunity_counts_without_zero_level_flat_bonus() {
        assert_eq!(threshold("hunger", &[1, 0, 1, 0]), Some(4));
        assert_eq!(threshold("hunger", &[2, 0, 0, 0]), Some(3));
        assert!(immune("poison", 1, &[1, 1]));
        assert_eq!(threshold("hunger", &[i32::MAX]), None);
        assert_eq!(threshold("poison", &[1, -1]), None);
        for effect in ["wither", "custom:poison", "minecraft:minecraft:poison"] {
            assert!(!immune(effect, 0, &[100]));
        }
    }
    #[test]
    fn source_gate_rejects_changed_thresholds() {
        assert!(known(Some(DEFINITION.as_bytes())));
        let mut value: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        value["modules"][0]["max_level"]["each_level"] = serde_json::json!(2);
        assert!(!known(Some(&serde_json::to_vec(&value).unwrap())));
        assert!(!known(None));
    }
}

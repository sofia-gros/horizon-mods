// Pinned Crystalstrike's equipment attributes and RestrictAngle rule.
pub(super) const ID: &str = "tconstruct:crystalstrike";

pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes.and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok()).is_some_and(|v| v == serde_json::json!({
        "level_display":"tconstruct:default", "modules":[
            {"type":"tconstruct:attribute","attribute":"minecraft:generic.attack_speed","each_level":0.05,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"},
            {"type":"tconstruct:attribute","attribute":"tconstruct:generic.bad_effect_duration_multiplier","each_level":0.05,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"},
            {"type":"tconstruct:armor_level","allow_broken":false,"held_tag":"tconstruct:modifiable/armor/held","key":"tconstruct:crystalstrike_knockback"}
        ]
    }))
}

pub(super) fn amount(level: i32) -> Option<f64> {
    (level >= 0).then(|| f64::from(0.0_f32 + 0.05_f32 * level as f32))
}

pub(super) fn multiplier(levels: &[i32]) -> Option<f64> {
    levels
        .iter()
        .try_fold(1.0, |value, level| Some(value * (1.0 + amount(*level)?)))
        .filter(|value| value.is_finite())
}

pub(super) fn duration(ticks: i32, multiplier: f64) -> Option<i32> {
    if !multiplier.is_finite() || multiplier < 0.0 {
        return None;
    }
    Some(if ticks == -1 || multiplier == 1.0 {
        ticks
    } else {
        ((f64::from(ticks) * multiplier) as i32).max(1)
    })
}

/// Known 1.20.1 nonbeneficial, noninstant effects with the default milk cure.
/// Unsupported effects are left alone rather than guessing curative metadata.
pub(super) fn curable_bad_effect(id: &str) -> bool {
    matches!(
        id.strip_prefix("minecraft:").unwrap_or(id),
        "slowness"
            | "mining_fatigue"
            | "nausea"
            | "blindness"
            | "hunger"
            | "weakness"
            | "poison"
            | "wither"
            | "glowing"
            | "levitation"
            | "unluck"
            | "bad_omen"
            | "darkness"
    )
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

/// Source code uses 32/16/8/4 directions for levels 1/2/3/4+.
/// Its nearby comment says four at level one, but the executable formula wins.
pub(super) fn angle_delta(angle: f64, level: i32) -> Option<f32> {
    if level <= 0 || !angle.is_finite() {
        return None;
    }
    let directions = 1u32 << (6 - level.min(4));
    let increment = std::f64::consts::TAU / f64::from(directions);
    // Java Math.round, including negative half ties.
    Some(((angle / increment + 0.5).floor() * increment - angle) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_piece_multiplies_separately_and_rounds_the_amount_as_float() {
        let one = f64::from(0.05_f32);
        assert_eq!(multiplier(&[1, 1]), Some((1.0 + one) * (1.0 + one)));
        assert_ne!(multiplier(&[1, 1]), multiplier(&[2]));
        assert_eq!(multiplier(&[-1]), None);
        assert_eq!(duration(100, 1.25), Some(125));
        assert_eq!(duration(-1, 1.25), Some(-1));
        assert_eq!(duration(0, 1.25), Some(1));
    }
    #[test]
    fn source_nonbeneficial_effects_include_neutral_glowing_but_not_new_effects() {
        assert!(curable_bad_effect("minecraft:glowing"));
        assert!(curable_bad_effect("poison"));
        for id in ["speed", "instant_damage", "infested", "custom:poison"] {
            assert!(!curable_bad_effect(id));
        }
    }
    #[test]
    fn higher_levels_coarsen_the_angle_grid_and_negative_ties_round_up() {
        let step = std::f64::consts::TAU / 32.0;
        assert_eq!(angle_delta(-step / 2.0, 1), Some((step / 2.0) as f32));
        assert_eq!(angle_delta(0.3, 4), angle_delta(0.3, i32::MAX));
        assert_eq!(angle_delta(f64::NAN, 1), None);
        assert_eq!(angle_delta(0.3, 0), None);
    }
}

/// Bounded held-armor classes. Other tools may expose the source attributes,
/// but their held Crystalstrike behavior is not claimed by this adapter.
pub(super) fn held_shield(item: &str) -> bool {
    matches!(
        item,
        "tconstruct:travelers_shield" | "tconstruct:plate_shield" | "tconstruct:battlesign"
    )
}

#[cfg(test)]
mod held_tests {
    use super::*;
    #[test]
    fn held_scope_is_the_three_stock_shields() {
        for item in [
            "tconstruct:travelers_shield",
            "tconstruct:plate_shield",
            "tconstruct:battlesign",
        ] {
            assert!(held_shield(item));
            assert_eq!(armor_slot(item), None);
        }
        assert!(!held_shield("tconstruct:slime_helmet"));
        assert!(!held_shield("tconstruct:sword"));
        assert!(!held_shield("minecraft:shield"));
    }
    #[test]
    fn two_hands_and_worn_pieces_multiply_independently() {
        let factor = 1.0 + amount(1).unwrap();
        assert_eq!(
            multiplier(&[1, 0, 0, 0, 1, 1]),
            Some(factor * factor * factor)
        );
        assert_ne!(
            multiplier(&[0, 0, 0, 0, 1, 1]),
            multiplier(&[0, 0, 0, 0, 2, 0])
        );
    }
}

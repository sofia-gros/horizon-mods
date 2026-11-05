// Source Dragonfall attributes and ModifierEvents.onCritical.
pub(super) const ID: &str = "tconstruct:dragonfall";
const DEFINITION: &str = r#"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:attribute","attribute":"tconstruct:player.critical_damage","each_level":0.1,"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"},{"type":"tconstruct:attribute","attribute":"tconstruct:generic.safe_fall_distance","each_level":2.0,"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"]}]}"#;
pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(DEFINITION).ok())
}
fn sum(levels: &[i32], each: f32) -> Option<f64> {
    levels.iter().try_fold(0.0, |total, level| {
        if *level < 0 {
            return None;
        }
        Some(total + f64::from(0.0_f32 + each * *level as f32))
    })
}
pub(super) fn safe_fall_bonus(levels: &[i32]) -> Option<f64> {
    sum(levels, 2.0).map(|value| value.clamp(-10.0, 100.0))
}
pub(super) fn critical(
    multiplier: f32,
    current: bool,
    airborne: bool,
    levels: &[i32],
) -> Option<(bool, f32)> {
    if !multiplier.is_finite() || multiplier < 0.0 {
        return None;
    }
    let boost = (1.5 + sum(levels, 0.1)?).clamp(0.0, 100.0) - 1.5;
    if boost <= 0.0 || !(current || airborne) {
        return Some((current, multiplier));
    }
    // A forced nonvanilla crit starts at its original 1.0, not 1.5.
    Some((true, (f64::from(multiplier) + boost) as f32))
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
    fn vanilla_and_forced_crit_keep_their_different_base_multipliers() {
        assert_eq!(critical(1.5, true, false, &[1]), Some((true, 1.6)));
        assert_eq!(critical(1.0, false, true, &[1]), Some((true, 1.1)));
        assert_eq!(critical(1.0, false, false, &[1]), Some((false, 1.0)));
        assert_eq!(critical(1.0, false, true, &[0]), Some((false, 1.0)));
        assert_eq!(critical(1.5, true, false, &[-1]), None);
    }
    #[test]
    fn slot_additions_and_source_caps() {
        assert_eq!(safe_fall_bonus(&[1, 2, 0]), Some(6.0));
        assert_eq!(safe_fall_bonus(&[i32::MAX]), Some(100.0));
        assert_eq!(critical(1.5, true, false, &[i32::MAX]), Some((true, 100.0)));
        assert_eq!(safe_fall_bonus(&[-1]), None);
    }
    #[test]
    fn source_gate_rejects_changed_modules() {
        assert!(known(Some(DEFINITION.as_bytes())));
        let mut value: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        value["modules"][0]["each_level"] = serde_json::json!(0.2);
        assert!(!known(Some(&serde_json::to_vec(&value).unwrap())));
        assert!(!known(None));
    }
}

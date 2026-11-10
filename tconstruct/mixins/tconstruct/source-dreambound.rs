// Dreambound's worn gravity attribute only; extra-jump runtime is separate work.
pub(super) const ID: &str = "tconstruct:dreambound";
const DEFINITION: &str = r#"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:attribute","attribute":"tconstruct:player.jump_count","flat":1.0,"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"]},{"type":"tconstruct:attribute","attribute":"forge:entity_gravity","flat":0.15,"modifier_level":1,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"}]}"#;
pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(DEFINITION).ok())
}
/// Source condition modifier_level: 1 is the exact logical level, not >= 1.
/// Flat 0.15 does not scale with level and is widened from a source float.
pub(super) fn gravity(level: i32) -> Option<f64> {
    if level < 0 {
        return None;
    }
    Some(if level == 1 { f64::from(0.15_f32) } else { 0.0 })
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
    fn gravity_exists_only_at_exact_level_one() {
        assert_eq!(gravity(0), Some(0.0));
        assert_eq!(gravity(1), Some(f64::from(0.15_f32)));
        assert_eq!(gravity(2), Some(0.0));
        assert_eq!(gravity(i32::MAX), Some(0.0));
        assert_eq!(gravity(-1), None);
    }
    #[test]
    fn changed_level_condition_does_not_inherit_stock_behavior() {
        assert!(known(Some(DEFINITION.as_bytes())));
        let mut value: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        value["modules"][1]["modifier_level"] = serde_json::json!({"min":1});
        assert!(!known(Some(&serde_json::to_vec(&value).unwrap())));
        assert!(!known(None));
    }
}

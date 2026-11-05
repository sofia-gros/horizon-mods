// Source Dreambound's independent worn gravity and extra-jump attributes.
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
/// The jump-count module is flat, independent of modifier level and gravity.
pub(super) fn extra_jumps(levels: &[i32; 4]) -> Option<u8> {
    if levels.iter().any(|level| *level < 0) {
        return None;
    }
    Some(levels.iter().filter(|level| **level > 0).count() as u8)
}
pub(super) fn fresh_jump(jumping: bool, was_jumping: bool, auto_jump: bool) -> bool {
    jumping && !was_jumping && !auto_jump
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
    fn holding_jump_and_auto_jump_do_not_spend_extra_jumps() {
        assert!(fresh_jump(true, false, false));
        assert!(!fresh_jump(true, true, false));
        assert!(!fresh_jump(false, true, false));
        assert!(!fresh_jump(true, false, true));
    }
    #[test]
    fn jump_count_is_flat_per_piece_at_any_positive_level() {
        assert_eq!(extra_jumps(&[0, 0, 0, 0]), Some(0));
        assert_eq!(extra_jumps(&[1, 2, 10, i32::MAX]), Some(4));
        assert_eq!(extra_jumps(&[0, 2, 0, 0]), Some(1));
        assert_eq!(extra_jumps(&[-1, 0, 0, 0]), None);
    }
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

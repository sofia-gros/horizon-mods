// Pinned OvergrowthModule: one point, 5% per effective level, once a second.
pub(super) const ID: &str = "tconstruct:overgrowth";
pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes.and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok()).is_some_and(|v| v == serde_json::json!({
        "level_display":"tconstruct:default", "modules":[{"type":"tconstruct:overgrowth","chance":{"each_level":0.05}}]
    }))
}
/// Use the native holder clock, including signed wrapping, not a second counter.
pub(super) fn second(tick_count: i32) -> bool {
    tick_count % 20 == 0
}
pub(super) fn active_slot(slot: i32, using: Option<usize>) -> bool {
    using.is_some_and(|used| {
        if slot == -106 {
            used == 40
        } else {
            slot >= 0 && slot < 36 && used == slot as usize
        }
    })
}
pub(super) fn eligible(current: i32, capacity: i32, level: i32) -> bool {
    level > 0 && current >= 0 && current < capacity && capacity <= 32767
}
pub(super) fn recover(
    current: i32,
    capacity: i32,
    level: i32,
    overworked: i32,
    random: f32,
) -> Option<i32> {
    (eligible(current, capacity, level)
        && overworked >= 0
        && random >= 0.0
        && random < 1.0
        && random < 0.05_f32 * level as f32)
        .then(|| {
            current
                .checked_add(overworked.checked_add(1)?)
                .map(|n| n.min(capacity))
        })
        .flatten()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_holder_clock_and_signed_wrap_keep_source_modulo() {
        for tick in [0, 20, 40, -20] { assert!(second(tick)); }
        for tick in [1, 19, 21, -1, i32::MIN, i32::MAX] { assert!(!second(tick)); }
        // A stack moved into a different holder follows that holder, not old item history.
        assert!(second(40));
        assert!(!second(41));
    }
    #[test]
    fn active_main_and_offhand_only_not_equal_stacks_elsewhere() {
        assert!(active_slot(2, Some(2)));
        assert!(active_slot(-106, Some(40)));
        for slot in [0, 1, 3, 100, 101, 102, 103, -106] {
            assert!(!active_slot(slot, Some(2)));
        }
        assert!(!active_slot(2, None));
        assert!(!active_slot(100, Some(100)));
    }
    #[test]
    fn source_float_chance_and_one_point_cap() {
        assert_eq!(recover(3, 50, 1, 0, 0.049), Some(4));
        assert_eq!(recover(3, 50, 1, 0, 0.05), None);
        assert_eq!(recover(49, 50, 2, 0, 0.099), Some(50));
        assert_eq!(recover(49, 50, 2, 0, 0.1), None);
        assert_eq!(recover(0, 50, 20, 0, 0.999), Some(1));
        assert_eq!(recover(0, 50, 100, 0, 0.999), Some(1));
    }
    #[test]
    fn overworked_bonus_clamps_to_capacity_without_overflow() {
        assert_eq!(recover(3, 50, 1, 2, 0.0), Some(6));
        assert_eq!(recover(49, 50, 1, 2, 0.0), Some(50));
        assert_eq!(recover(0, 50, 1, -1, 0.0), None);
        assert_eq!(recover(0, 50, 1, i32::MAX, 0.0), None);
    }
    #[test]
    fn full_invalid_and_absent_recovery_is_inert() {
        for (n, cap, level) in [
            (50, 50, 1),
            (51, 50, 1),
            (-1, 50, 1),
            (0, 0, 1),
            (0, 50, 0),
            (0, 50, -1),
            (0, 32768, 1),
        ] {
            assert!(!eligible(n, cap, level));
            assert_eq!(recover(n, cap, level, 0, 0.0), None);
        }
        for r in [f32::NAN, f32::INFINITY, -0.1, 1.0] {
            assert_eq!(recover(0, 50, 1, 0, r), None);
        }
    }
    #[test]
    fn exact_definition_only_and_reload_sensitive() {
        let bytes=br#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:overgrowth","chance":{"each_level":0.05}}]}"#;
        assert!(known(Some(bytes)));
        assert!(!known(None));
        assert!(!known(Some(b"{}")));
        let mut changed: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        changed["modules"][0]["chance"]["each_level"] = serde_json::json!(0.1);
        assert!(!known(Some(&serde_json::to_vec(&changed).unwrap())));
        assert!(known(Some(bytes)));
    }
}

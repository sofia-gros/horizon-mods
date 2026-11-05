pub(super) const ID: &str = "tconstruct:strong_bones";

pub(super) fn milk_duration(level: i32) -> Option<i32> {
    if level <= 0 {
        return None;
    }
    600_i32.checked_mul(level)?.checked_add(600)
}

pub(super) fn replaces_active(
    duration: i32,
    amplifier: i32,
    incoming_duration: i32,
    incoming_amplifier: i32,
) -> bool {
    incoming_amplifier > amplifier
        || (incoming_amplifier == amplifier
            && duration != -1
            && (incoming_duration == -1 || incoming_duration > duration))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn milk_grants_thirty_seconds_plus_thirty_per_level() {
        assert_eq!(milk_duration(1), Some(1200));
        assert_eq!(milk_duration(2), Some(1800));
        assert_eq!(milk_duration(0), None);
        assert_eq!(milk_duration(i32::MAX), None);
    }
    #[test]
    fn ownership_changes_only_when_the_active_instance_is_replaced() {
        assert!(replaces_active(100, 0, 50, 1));
        assert!(replaces_active(100, 0, 200, 0));
        assert!(replaces_active(100, 0, -1, 0));
        assert!(!replaces_active(-1, 0, 200, 0));
        assert!(!replaces_active(100, 0, 100, 0));
        assert!(!replaces_active(100, 1, 200, 0));
    }
}

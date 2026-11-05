/// Source Lacerating is a statically registered Java modifier. Any dynamic
/// definition at this ID is outside the supported stock implementation.
pub(super) fn supported_definition(world: &super::tconstruct_world::World) -> bool {
    world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok())
        .is_some_and(|raw| !raw.0.contains_key("tconstruct:tinkering/modifiers/lacerating"))
}

pub(super) fn duration(level: i32, looting: i32, existing: Option<(i32, i32)>) -> Option<i32> {
    if level <= 0 || looting < 0 { return None; }
    let base = level.checked_mul(40)?;
    match existing {
        Some((ticks, amplifier)) if amplifier == looting && ticks > 0 => base.checked_add(ticks),
        // Retain the native infinite effect instead of turning it into a timer.
        Some((-1, amplifier)) if amplifier == looting => Some(-1),
        _ => base.checked_add(19),
    }
}

pub(super) fn damage_tick(remaining: i32) -> bool { remaining > 0 && remaining % 20 == 0 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_duration_extends_the_existing_clock_without_a_second_delay() {
        assert_eq!(duration(1, 0, None), Some(59));
        assert_eq!(duration(2, 0, None), Some(99));
        assert_eq!(duration(1, 0, Some((37, 0))), Some(77));
        assert_eq!(duration(1, 1, Some((37, 0))), Some(59));
        assert_eq!(duration(1, 0, Some((-1, 0))), Some(-1));
        assert_eq!(duration(0, 0, None), None);
        assert_eq!(duration(i32::MAX, 0, None), None);
    }
    #[test]
    fn damage_is_once_per_second_not_scaled_by_amplifier() {
        assert!(damage_tick(40));
        assert!(damage_tick(20));
        for ticks in [-1, 0, 1, 19, 21, 59] { assert!(!damage_tick(ticks)); }
    }
}

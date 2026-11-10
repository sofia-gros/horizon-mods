// Pinned InsatiableModifier: a successful charged hit grants one stack for 100 ticks.
pub(super) const EFFECT: &str = "tconstruct:insatiable_melee";
pub(super) const DURATION: i32 = 100;

pub(super) fn next_amplifier(current: Option<i32>) -> i32 {
    current.unwrap_or(-1).saturating_add(1).min(4)
}

pub(super) fn charged(scale: f32) -> bool {
    scale.is_finite() && scale > 0.9
}

pub(super) fn bonus(amplifier: Option<i32>, level: i32, multiplier: f32) -> Option<f32> {
    if level <= 0 || !multiplier.is_finite() || multiplier < 0.0 {
        return None;
    }
    let effect_level = amplifier?.checked_add(1)?;
    if effect_level <= 0 {
        return None;
    }
    let value = effect_level as f32 * level as f32 / 2.0 * multiplier;
    value.is_finite().then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_hits_cap_and_refresh_without_being_modifier_level_dependent() {
        let mut amp = None;
        for expected in [0, 1, 2, 3, 4, 4] {
            amp = Some(next_amplifier(amp));
            assert_eq!(amp, Some(expected));
        }
        assert_eq!(DURATION, 100);
        assert_eq!(next_amplifier(None), 0);
    }
    #[test]
    fn source_flat_bonus_uses_effect_level_and_tool_multiplier() {
        assert_eq!(bonus(None, 2, 1.0), None);
        assert_eq!(bonus(Some(0), 1, 1.0), Some(0.5));
        assert_eq!(bonus(Some(4), 2, 1.25), Some(6.25));
        assert_eq!(bonus(Some(0), 0, 1.0), None);
        assert_eq!(bonus(Some(0), 1, f32::NAN), None);
        assert!(!charged(0.9));
        assert!(charged(0.91));
    }
}

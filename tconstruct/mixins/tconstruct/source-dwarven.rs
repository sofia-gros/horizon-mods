// DwarvenModifier.getBoost at TConstruct f4e77d43.
// `bonus` is the source stat multiplier times 6.0 for mining or 0.05 for velocity.
pub(super) fn boost(y: f32, max_build_height: i32, level: i32, base: f32, bonus: f32) -> f32 {
    if y < 64.0 {
        let scale = ((64.0 - y) / 64.0).clamp(0.0, 2.0);
        return base + level.max(0) as f32 * scale * bonus;
    }
    let baseline = (max_build_height as f32 - 192.0).max(96.0);
    if y > baseline {
        if y >= baseline + 128.0 { return base * 0.25; }
        return base * (1.0 - (y - baseline) / 128.0 * 0.75);
    }
    base
}

#[cfg(test)]
mod tests {
    use super::boost;
    #[test]
    fn depth_boost_and_high_altitude_penalty_match_source() {
        assert_eq!(boost(64.0, 320, 1, 4.0, 6.0), 4.0);
        assert_eq!(boost(0.0, 320, 1, 4.0, 6.0), 10.0);
        assert_eq!(boost(-64.0, 320, 1, 4.0, 6.0), 16.0);
        assert_eq!(boost(-256.0, 320, 1, 4.0, 6.0), 16.0);
        assert_eq!(boost(128.0, 320, 1, 4.0, 6.0), 4.0);
        assert_eq!(boost(192.0, 320, 1, 4.0, 6.0), 2.5);
        assert_eq!(boost(256.0, 320, 1, 4.0, 6.0), 1.0);
    }
    #[test]
    fn source_uses_same_height_curve_for_velocity_and_short_worlds() {
        assert!((boost(0.0, 128, 2, 1.0, 0.05) - 1.1).abs() < 0.00001);
        assert_eq!(boost(96.0, 128, 1, 4.0, 6.0), 4.0);
        assert_eq!(boost(224.0, 128, 1, 4.0, 6.0), 1.0);
    }
}

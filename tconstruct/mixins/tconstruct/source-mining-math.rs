// Original stat boosts and effective-block conditional mining modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Conditional { Jagged(i32), Stonebound(i32), Maintained(i32) }

pub(super) fn base_speed(base: f32, add_scale: f32, lightweight: i32, heavy: i32, vintage: i32, entangled: bool) -> f32 {
    ((base + if entangled { 6.0 * add_scale } else { 0.0 })
        * (1.0 + 0.08 * lightweight.max(0) as f32)
        * (1.0 + 0.15 * heavy.max(0) as f32)
        * (1.0 - 0.1 * vintage.max(0) as f32)).clamp(0.1, 2048.0)
}

/// Conditionals are applied in first material-trait insertion order.
pub(super) fn conditional_speed(mut speed: f32, loss: u32, maximum: u32, durability_all: f32,
    mining_all: f32, modifiers: &[Conditional]) -> f32 {
    let wear = (loss as f32 / durability_all.max(f32::MIN_POSITIVE)).sqrt() * 0.01;
    let half = maximum as f32 * 0.5;
    let remaining = maximum.saturating_sub(loss) as f32;
    let maintained = if half > 0.0 { ((remaining - half) / half).clamp(0.0, 1.0) } else { 0.0 };
    for modifier in modifiers {
        match *modifier {
            Conditional::Jagged(level) => speed *= 1.0 - wear * level.max(0) as f32,
            Conditional::Stonebound(level) => speed *= 1.0 + wear * level.max(0) as f32,
            Conditional::Maintained(level) => speed += maintained * level.max(0) as f32 * 6.0 * mining_all,
        }
    }
    speed.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_static_operations_and_clamp() {
        assert!((base_speed(3.5, 1.0, 1, 1, 1, false) - 3.9123).abs() < 0.0001);
        assert!((base_speed(3.5, 1.2, 0, 0, 0, true) - 10.7).abs() < 0.0001);
        assert_eq!(base_speed(1.0, 1.0, 0, 0, 20, false), 0.1);
    }
    #[test]
    fn source_effective_wear_and_order() {
        assert_eq!(conditional_speed(4.0, 0, 100, 1.0, 1.0, &[Conditional::Jagged(1)]), 4.0);
        assert!((conditional_speed(4.0, 100, 100, 1.0, 1.0, &[Conditional::Jagged(1)]) - 3.6).abs() < 0.0001);
        assert!((conditional_speed(4.0, 25, 100, 1.0, 1.0, &[Conditional::Stonebound(1)]) - 4.2).abs() < 0.0001);
        assert!((conditional_speed(4.0, 25, 100, 1.0, 1.0, &[Conditional::Maintained(1)]) - 7.0).abs() < 0.0001);
        assert_ne!(conditional_speed(4.0, 25, 100, 1.0, 1.0, &[Conditional::Jagged(1), Conditional::Maintained(1)]),
            conditional_speed(4.0, 25, 100, 1.0, 1.0, &[Conditional::Maintained(1), Conditional::Jagged(1)]));
    }
}

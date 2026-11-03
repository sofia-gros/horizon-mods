// Original stat boosts and effective-block conditional mining modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Conditional { Jagged(i32), Stonebound(i32), Maintained(i32), Dwarven(i32), Crumbling(i32), Overwield(i32, i32) }

pub(super) fn base_speed(base: f32, add_scale: f32, lightweight: i32, heavy: i32, vintage: i32, entangled: bool) -> f32 {
    base_speed_unclamped(base, add_scale, lightweight, heavy, vintage, entangled).clamp(0.1, 2048.0)
}

pub(super) fn base_speed_unclamped(base: f32, add_scale: f32, lightweight: i32, heavy: i32, vintage: i32, entangled: bool) -> f32 {
    (base + if entangled { 6.0 * add_scale } else { 0.0 })
        * (1.0 + 0.08 * lightweight.max(0) as f32)
        * (1.0 + 0.15 * heavy.max(0) as f32)
        * (1.0 - 0.1 * vintage.max(0) as f32)
}

/// Source priority: maintained (100) before percent traits (75), stable within a priority.
pub(super) fn conditional_speed(speed: f32, loss: u32, maximum: u32, durability_all: f32,
    mining_all: f32, modifiers: &[Conditional]) -> f32 {
    conditional_speed_at(speed, loss, maximum, durability_all, mining_all, modifiers, None)
}

pub(super) fn conditional_speed_at(speed: f32, loss: u32, maximum: u32, durability_all: f32,
    mining_all: f32, modifiers: &[Conditional], position: Option<(i32, i32)>) -> f32 {
    conditional_speed_for(speed, loss, maximum, durability_all, mining_all, modifiers, position, true, false)
}

pub(super) fn conditional_speed_for(mut speed: f32, loss: u32, maximum: u32, durability_all: f32,
    mining_all: f32, modifiers: &[Conditional], position: Option<(i32, i32)>, effective: bool, tool_not_required: bool) -> f32 {
    let wear = (loss as f32 / durability_all.max(f32::MIN_POSITIVE)).sqrt() * 0.01;
    let half = maximum as f32 * 0.5;
    let remaining = maximum.saturating_sub(loss) as f32;
    let maintained = if half > 0.0 { ((remaining - half) / half).clamp(0.0, 1.0) } else { 0.0 };
    for modifier in modifiers {
        match *modifier {
            Conditional::Maintained(level) if effective => speed += maintained * level.max(0) as f32 * 6.0 * mining_all,
            Conditional::Crumbling(level) if tool_not_required => speed += level.max(0) as f32 * mining_all,
            _ => {},
        }
    }
    if let (true, Some((y, max_height))) = (effective, position) {
        for modifier in modifiers {
            if let Conditional::Dwarven(level) = *modifier {
                speed = super::tconstruct_source_dwarven::boost(y as f32, max_height, level, speed, 6.0 * mining_all);
            }
        }
    }
    for modifier in modifiers {
        match *modifier {
            Conditional::Jagged(level) if effective => speed *= 1.0 - wear * level.max(0) as f32,
            Conditional::Stonebound(level) if effective => speed *= 1.0 + wear * level.max(0) as f32,
            Conditional::Overwield(level, overslime) if effective => speed *= 1.0 + 0.2 * level.max(0).min(overslime.max(0)) as f32,
            _ => {},
        }
    }
    speed.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crumbling_handles_ineffective_and_required_tool_blocks() {
        let effects = [Conditional::Crumbling(2), Conditional::Maintained(1), Conditional::Jagged(1)];
        // Ineffective but no tool required: only Crumbling is active.
        assert_eq!(conditional_speed_for(1.0, 25, 100, 1.0, 1.5, &effects, None, false, true), 4.0);
        assert_eq!(conditional_speed_for(1.0, 25, 100, 1.0, 1.5, &effects, None, false, false), 1.0);
        // Effective: flat boosts precede the percentage-based trait.
        let actual = conditional_speed_for(4.0, 25, 100, 1.0, 1.0, &effects, None, true, true);
        assert!((actual - 8.55).abs() < 0.0001);
        let required = conditional_speed_for(4.0, 25, 100, 1.0, 1.0, &effects, None, true, false);
        assert!((required - 6.65).abs() < 0.0001);
    }
    #[test]
    fn dwarven_runs_between_maintained_and_percent_traits() {
        let effects = [Conditional::Jagged(1), Conditional::Dwarven(1), Conditional::Maintained(1)];
        let actual = conditional_speed_at(4.0, 25, 100, 1.0, 1.0, &effects, Some((0, 320)));
        assert!((actual - 12.35).abs() < 0.0001);
        let high = conditional_speed_at(4.0, 25, 100, 1.0, 1.0, &effects, Some((256, 320)));
        assert!((high - 1.6625).abs() < 0.0001);
    }
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
        let mixed = conditional_speed(4.0, 25, 100, 1.0, 1.0, &[Conditional::Jagged(1), Conditional::Maintained(1)]);
        assert!((mixed - 6.65).abs() < 0.0001);
        assert_eq!(mixed, conditional_speed(4.0, 25, 100, 1.0, 1.0, &[Conditional::Maintained(1), Conditional::Jagged(1)]));
        let mixed_stone = conditional_speed(4.0, 25, 100, 1.0, 1.0, &[Conditional::Stonebound(1), Conditional::Maintained(1)]);
        assert!((mixed_stone - 7.35).abs() < 0.0001);
        assert_eq!(mixed_stone, conditional_speed(4.0, 25, 100, 1.0, 1.0, &[Conditional::Maintained(1), Conditional::Stonebound(1)]));
    }
}

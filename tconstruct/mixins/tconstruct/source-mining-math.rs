// Original stat boosts and effective-block conditional mining modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Conditional { Jagged(i32), Stonebound(i32), Maintained(i32), Dwarven(i32), Crumbling(i32), Blasting(i32), Hydraulic(i32), Overwield(i32, i32, bool), Momentum(i32, i32) }

pub(super) fn base_speed(base: f32, add_scale: f32, lightweight: i32, heavy: i32, vintage: i32, entangled: bool) -> f32 {
    base_speed_unclamped(base, add_scale, lightweight, heavy, vintage, entangled).clamp(0.1, 2048.0)
}

pub(super) fn base_speed_unclamped(base: f32, add_scale: f32, lightweight: i32, heavy: i32, vintage: i32, entangled: bool) -> f32 {
    (base + if entangled { 6.0 * add_scale } else { 0.0 })
        * (1.0 + 0.08 * lightweight.max(0) as f32)
        * (1.0 + 0.15 * heavy.max(0) as f32)
        * (1.0 - 0.1 * vintage.max(0) as f32)
}

// Snapshots store whole trait levels and upgrade progress in fixed units.
// Match IncrementalModifierEntry.scaleLevel after reconstructing logical level.
fn incremental_level(amount: i32, needed: i32) -> f32 {
    let amount = amount.max(0);
    let whole = amount / needed;
    let remainder = amount % needed;
    if remainder == 0 { whole as f32 }
    else { (whole + 1) as f32 + (remainder - needed) as f32 / needed as f32 }
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

pub(super) fn conditional_speed_for(speed: f32, loss: u32, maximum: u32, durability_all: f32,
    mining_all: f32, modifiers: &[Conditional], position: Option<(i32, i32)>, effective: bool, tool_not_required: bool) -> f32 {
    conditional_speed_for_resistance(speed, loss, maximum, durability_all, mining_all, modifiers, position, effective, tool_not_required, None)
}

pub(super) fn conditional_speed_for_resistance(speed: f32, loss: u32, maximum: u32, durability_all: f32,
    mining_all: f32, modifiers: &[Conditional], position: Option<(i32, i32)>, effective: bool, tool_not_required: bool, resistance: Option<f32>) -> f32 {
    conditional_speed_with_water(speed, loss, maximum, durability_all, mining_all, modifiers, position, effective, tool_not_required, resistance, None)
}

/// `water` is Some(aqua_affinity) only when the actor's eyes are in water.
pub(super) fn conditional_speed_with_water(mut speed: f32, loss: u32, maximum: u32, durability_all: f32,
    mining_all: f32, modifiers: &[Conditional], position: Option<(i32, i32)>, effective: bool, tool_not_required: bool, resistance: Option<f32>, water: Option<bool>) -> f32 {
    // Preserve the existing invalid-durability floor; source factor evaluation
    // multiplies the square root by level before the signed coefficient.
    let wear = (loss as f32 / durability_all.max(f32::MIN_POSITIVE)).sqrt();
    let half = maximum as f32 * 0.5;
    let remaining = maximum.saturating_sub(loss) as f32;
    let maintained = if half > 0.0 { ((remaining - half) / half).clamp(0.0, 1.0) } else { 0.0 };
    for modifier in modifiers {
        match *modifier {
            Conditional::Maintained(level) if effective => speed += maintained * level.max(0) as f32 * 6.0 * mining_all,
            Conditional::Crumbling(level) if tool_not_required => speed += level.max(0) as f32 * mining_all,
            Conditional::Hydraulic(amount) if effective => {
                if let Some(aqua) = water {
                    speed += (mining_all * if aqua { 8.0 } else { 40.0 }) * incremental_level(amount, 36);
                }
            },
            Conditional::Blasting(amount) if effective => {
                if let Some(resistance) = resistance.filter(|v| v.is_finite()) {
                    // PostFixOperator.POWER widens its float operands for
                    // Math.pow, then narrows before subsequent float operators.
                    let exponent = (6.0 - resistance) / 1.5;
                    let power = 3.0f64.powf(f64::from(exponent)) as f32;
                    speed += power.min(10.0) * incremental_level(amount, 20) * mining_all;
                }
            },
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
            Conditional::Jagged(level) if effective => speed *= 1.0 + (wear * level.max(0) as f32) * -0.01,
            Conditional::Stonebound(level) if effective => speed *= 1.0 + (wear * level.max(0) as f32) * 0.01,
            Conditional::Momentum(level, effect) if effective => speed *= 1.0 + (level.max(0) as f32 * effect.max(0) as f32) / 40.0,
            Conditional::Overwield(level, overslime, broad) if effective => {
                let cost = if broad { 5 } else { 1 };
                let used = overslime.max(0).min(level.max(0).saturating_mul(cost));
                // The broad-tool RPN divides the consumed amount by five
                // before multiplying by 0.2; reassociation changes f32 bits.
                speed *= 1.0 + (used as f32 / cost as f32) * 0.2;
            },
            _ => {},
        }
    }
    speed.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hydraulic_uses_source_float_operation_order() {
        let amount = 1;
        let multiplier = 0.03f32;
        let level = 1.0f32 + (amount - 36) as f32 / 36.0;
        let actual = conditional_speed_with_water(0.0, 0, 100, 1.0, multiplier, &[Conditional::Hydraulic(amount)], None, true, false, None, Some(false));
        let source_order = (multiplier * 40.0) * level;
        let reordered = (40.0 * level) * multiplier;
        assert_eq!(actual.to_bits(), source_order.to_bits());
        assert_ne!(source_order.to_bits(), reordered.to_bits());
    }
    #[test]
    fn blasting_uses_resistance_incremental_level_and_order() {
        let effects = [Conditional::Stonebound(1), Conditional::Blasting(10)];
        let calculate = |resistance, effective| conditional_speed_for_resistance(
            4.0, 25, 100, 1.0, 2.0, &effects, None, effective, false, resistance);
        // Resistance 3 produces 9 extra at half a level with multiplier 2;
        // the flat boost precedes Stonebound's 1.05 multiplier.
        assert!((calculate(Some(3.0), true) - 13.65).abs() < 0.0001);
        assert_eq!(calculate(Some(0.0), false), 4.0);
        assert!((calculate(Some(0.0), true) - 14.7).abs() < 0.0001); // cap 10
        assert!((calculate(Some(6.0), true) - 5.25).abs() < 0.0001);
        assert!((calculate(None, true) - 4.2).abs() < 0.0001);
        assert!((calculate(Some(f32::NAN), true) - 4.2).abs() < 0.0001);
    }
    #[test]
    fn overwield_uses_live_overslime_and_broad_cost() {
        for (level, amount, broad, expected) in [(2, 0, false, 10.0), (2, 1, false, 12.0), (2, 5, false, 14.0), (2, 1, true, 10.4), (2, 5, true, 12.0), (2, 10, true, 14.0)] {
            let modifiers = [Conditional::Overwield(level, amount, broad)];
            let speed = conditional_speed_for(10.0, 0, 100, 1.0, 1.0, &modifiers, None, true, false);
            assert!((speed - expected).abs() < 0.0001);
            assert_eq!(conditional_speed_for(10.0, 0, 100, 1.0, 1.0, &modifiers, None, false, false), 10.0);
        }
    }

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

#[cfg(test)]
mod momentum_tests {
    use super::*;
    #[test]
    fn momentum_follows_order_and_only_applies_to_effective_blocks() {
        let mods = [Conditional::Momentum(2, 10), Conditional::Overwield(1, 1, false)];
        let speed = conditional_speed_for(4.0, 0, 100, 1.0, 1.0, &mods, None, true, false);
        assert!((speed - 7.2).abs() < 0.00001);
        assert_eq!(conditional_speed_for(4.0, 0, 100, 1.0, 1.0, &mods, None, false, false), 4.0);
        assert_eq!(conditional_speed_for(4.0, 0, 100, 1.0, 1.0, &[Conditional::Momentum(1, 0)], None, true, false), 4.0);
    }
}

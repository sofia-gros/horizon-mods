// Supported source trait formulas. Other modifiers are not interpreted here.
pub(super) fn cultivated_repair_factor(level: i32) -> f32 {
    1.0 + level as f32 * 0.5
}

pub(super) fn jagged_damage_bonus(
    damage: f32,
    lost: u32,
    durability_multiplier: f32,
    level: i32,
) -> f32 {
    damage * (lost as f32 / durability_multiplier).sqrt() * level as f32 * 0.005
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_repair_and_jagged_effects_scale_with_levels_and_wear() {
        assert_eq!(cultivated_repair_factor(0), 1.0);
        assert_eq!(cultivated_repair_factor(2), 2.0);
        assert_eq!(jagged_damage_bonus(3.0, 0, 1.0, 1), 0.0);
        assert!((jagged_damage_bonus(3.0, 100, 1.0, 1) - 0.15).abs() < 0.00001);
        assert!((jagged_damage_bonus(3.0, 100, 4.0, 2) - 0.15).abs() < 0.00001);
    }
}

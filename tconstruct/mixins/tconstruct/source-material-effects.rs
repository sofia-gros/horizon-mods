// Supported source trait formulas. Other modifier effects are not interpreted here.
pub(super) fn repair_factor(cultivated: i32, preserved: i32) -> f32 {
    (1.0 + cultivated as f32 * 0.5) * (1.0 + preserved as f32 * 0.15)
}

pub(super) fn wear_damage_bonus(
    damage: f32,
    lost: u32,
    durability_multiplier: f32,
    jagged: i32,
    stonebound: i32,
) -> f32 {
    let wear = (lost as f32 / durability_multiplier).sqrt() * 0.005;
    damage * ((1.0 + jagged as f32 * wear) * (1.0 - stonebound as f32 * wear) - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_repair_and_wear_effects_scale_with_levels() {
        assert_eq!(repair_factor(0, 0), 1.0);
        assert_eq!(repair_factor(2, 0), 2.0);
        assert!((repair_factor(1, 1) - 1.725).abs() < 0.00001);
        assert_eq!(wear_damage_bonus(3.0, 0, 1.0, 1, 0), 0.0);
        assert!((wear_damage_bonus(3.0, 100, 1.0, 1, 0) - 0.15).abs() < 0.00001);
        assert!((wear_damage_bonus(3.0, 100, 1.0, 0, 1) + 0.15).abs() < 0.00001);
        assert!((wear_damage_bonus(3.0, 100, 1.0, 1, 1) + 0.0075).abs() < 0.00001);
    }
}

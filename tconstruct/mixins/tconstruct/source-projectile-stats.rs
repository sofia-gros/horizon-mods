// Original material projectile multipliers. Injection at firing remains pending.
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

fn stat(stack: &ItemStack, key: &str) -> Option<f32> {
    match stack.data()?.other("minecraft:custom_data")?.get(key)? {
        Tag::Float(bits) => Some(f32::from_bits(*bits)),
        _ => None,
    }
}

pub(crate) fn projectile_stats(stack: &ItemStack) -> Option<(f32, f32)> {
    if !matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow") { return None; }
    let velocity = stat(stack, "tic_velocity")?;
    let accuracy = stat(stack, "tic_accuracy")?;
    if !velocity.is_finite() || velocity < 0.0 || !accuracy.is_finite() || accuracy <= 0.0 { return None; }
    Some((velocity, 3.0 * (1.0 / accuracy - 1.0)))
}

/// Source Dwarven applies to the shooter's actual height at firing time.
pub(crate) fn projectile_stats_at(stack: &ItemStack, y: f32, max_height: i32) -> Option<(f32, f32)> {
    let (velocity, spread) = projectile_stats(stack)?;
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let level = match custom.get("tic_dwarven") { Some(Tag::Int(value)) => *value, _ => 0 };
    let multiplier = match custom.get("tic_dwarven_velocity_multiplier") {
        Some(Tag::Float(bits)) => f32::from_bits(*bits), _ => return Some((velocity, spread)),
    };
    if level <= 0 || !multiplier.is_finite() || !y.is_finite() { return Some((velocity, spread)); }
    Some((super::tconstruct_source_dwarven::boost(y, max_height, level, velocity, 0.05 * multiplier).clamp(0.0, 1024.0), spread))
}

pub(crate) fn projectile_bonus(stack: &ItemStack) -> f64 {
    if let Some(value) = stat(stack, "tic_projectile_damage") {
        if value.is_finite() { return (value as f64 - 2.0).max(-2.0); }
    }
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_upgrades")).and_then(|d| d.get("tconstruct:diamond")) {
        Some(Tag::Int(value)) if *value > 0 => 0.5,
        _ => 0.0,
    }
}

/// Pinned RestrictAngleModule rounds the horizontal direction after launch.
pub(crate) fn restrict_direction(direction: [f64; 3], level: i32) -> ([f64; 3], Option<f32>) {
    if level <= 0 { return (direction, None); }
    let count = 1u32 << (6 - level).clamp(2, 6);
    let increment = std::f64::consts::TAU / f64::from(count);
    let old = direction[0].atan2(direction[2]);
    // Java Math.round uses floor(value + 0.5), including negative ties.
    let new = (old / increment + 0.5).floor() * increment;
    let (sin, cos) = (new - old).sin_cos();
    ([direction[0] * cos + direction[2] * sin, direction[1], direction[2] * cos - direction[0] * sin], Some(new.to_degrees() as f32))
}

pub(crate) fn crystalbound_level(stack: &ItemStack) -> i32 {
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_crystalbound_angle")) {
        Some(Tag::Int(level)) => (*level).max(0), _ => 0,
    }
}

pub(crate) fn bow_charge(elapsed: u32, draw_ticks: u32) -> f32 {
    let f = elapsed as f32 / draw_ticks.max(1) as f32;
    ((f * f + f * 2.0) / 3.0).min(1.0)
}

pub(crate) fn crossbow_angle(index: usize, count: usize) -> f64 {
    (index as f64 - count.saturating_sub(1) as f64 / 2.0) * 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bow(id: &str, velocity: f32, accuracy: f32) -> ItemStack {
        let mut stack = ItemStack::new(id, 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_velocity", Tag::Float(velocity.to_bits())),
            ("tic_accuracy", Tag::Float(accuracy.to_bits())),
        ])));
        stack
    }
    #[test]
    fn dwarven_uses_shooter_height_for_both_bows() {
        for id in ["tconstruct:longbow", "tconstruct:crossbow"] {
            let mut stack = bow(id, 1.0, 1.0);
            stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
                ("tic_velocity", Tag::Float(1.0f32.to_bits())),
                ("tic_accuracy", Tag::Float(1.0f32.to_bits())),
                ("tic_dwarven", Tag::Int(1)),
                ("tic_dwarven_velocity_multiplier", Tag::Float(2.0f32.to_bits())),
            ])));
            assert!((projectile_stats_at(&stack, 0.0, 320).unwrap().0 - 1.1).abs() < 0.00001);
            assert!((projectile_stats_at(&stack, 256.0, 320).unwrap().0 - 0.25).abs() < 0.00001);
        }
    }
    #[test]
    fn diamond_damage_comes_from_the_firing_stack() {
        let mut stack = bow("tconstruct:crossbow", 1.0, 1.0);
        assert_eq!(projectile_bonus(&stack), 0.0);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:diamond", Tag::Int(1))]))])));
        assert_eq!(projectile_bonus(&stack), 0.5);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_upgrades", Tag::compound([("tconstruct:diamond", Tag::Int(1))])),
            ("tic_projectile_damage", Tag::Float(2.75f32.to_bits())),
        ])));
        assert_eq!(projectile_bonus(&stack), 0.75);
    }
    #[test]
    fn reads_source_stats_and_spread() {
        for id in ["tconstruct:longbow", "tconstruct:crossbow"] {
            let (v, spread) = projectile_stats(&bow(id, 1.25, 0.75)).unwrap();
            assert!((v - 1.25).abs() < 1e-6);
            assert!((spread - 1.0).abs() < 1e-6);
        }
    }
    #[test]
    fn source_fractional_charge() {
        let c = bow_charge(1, 14);
        assert!((c - 0.04931973).abs() < 1e-6);
        assert!(c * 2.5 >= 0.1);
        assert!((bow_charge(7, 14) - (1.25 / 3.0)).abs() < 1e-6);
        assert_eq!(bow_charge(14, 14), 1.0);
    }
    #[test]
    fn source_fan_angles() {
        assert_eq!((0..3).map(|i| crossbow_angle(i, 3)).collect::<Vec<_>>(), vec![-10.0, 0.0, 10.0]);
        assert_eq!(crossbow_angle(0, 1), 0.0);
    }
    #[test]
    fn crystalbound_rounds_horizontal_direction_after_spread() {
        let original=[0.12f64.sin(),0.4,0.12f64.cos()];
        assert_eq!(restrict_direction(original,0),(original,None));
        let (rounded,yaw)=restrict_direction(original,1);
        assert!((rounded[0].atan2(rounded[2])-std::f64::consts::PI/16.0).abs()<1e-9);
        assert_eq!(rounded[1],0.4);
        assert!((rounded[0]*rounded[0]+rounded[2]*rounded[2]-1.0).abs()<1e-9);
        assert!((yaw.unwrap()-11.25).abs()<0.001);
        let (coarse,_) = restrict_direction(original,4);
        assert!(coarse[0].abs()<1e-9);
    }
    #[test]
    fn excludes_vanilla_and_invalid_stats() {
        assert!(projectile_stats(&bow("minecraft:bow", 1.0, 1.0)).is_none());
        assert!(projectile_stats(&bow("tconstruct:longbow", 1.0, 0.0)).is_none());
        assert!(projectile_stats(&bow("tconstruct:crossbow", f32::NAN, 1.0)).is_none());
    }
}

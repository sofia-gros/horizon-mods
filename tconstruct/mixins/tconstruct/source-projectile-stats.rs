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

pub(crate) fn projectile_bonus(stack: &ItemStack) -> f64 {
    if let Some(value) = stat(stack, "tic_projectile_damage") {
        if value.is_finite() { return (value as f64 - 2.0).max(-2.0); }
    }
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_upgrades")).and_then(|d| d.get("tconstruct:diamond")) {
        Some(Tag::Int(value)) if *value > 0 => 0.5,
        _ => 0.0,
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
    fn excludes_vanilla_and_invalid_stats() {
        assert!(projectile_stats(&bow("minecraft:bow", 1.0, 1.0)).is_none());
        assert!(projectile_stats(&bow("tconstruct:longbow", 1.0, 0.0)).is_none());
        assert!(projectile_stats(&bow("tconstruct:crossbow", f32::NAN, 1.0)).is_none());
    }
}

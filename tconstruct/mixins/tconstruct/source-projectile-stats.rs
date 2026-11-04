// Original material projectile stats, read by both existing bow firing hooks.
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
    // ConditionalStatModifierHook clamps supported accuracy before inaccuracy.
    let accuracy = accuracy.clamp(0.1, 1.0);
    // Maintained (priority 100) precedes Dwarven (85), including its altitude
    // penalty. Evaluate current wear at release, never bake it into base stats.
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let level = match custom.get("tic_maintained") { Some(Tag::Int(value)) => (*value).max(0), _ => 0 };
    let maximum = match custom.get("tic_max_damage") { Some(Tag::Int(value)) if *value > 0 => *value as u32, _ => 0 };
    let multiplier = stat(stack, "tic_maintained_velocity_multiplier").unwrap_or(0.0);
    let velocity = if level > 0 && maximum > 0 && multiplier.is_finite() {
        let half = maximum as f32 * 0.5;
        let remaining = maximum.saturating_sub(stack.damage) as f32;
        velocity + ((remaining - half).max(0.0) / half) * level as f32 * 0.08 * multiplier
    } else { velocity };
    Some((velocity, 3.0 * (1.0 / accuracy - 1.0)))
}

/// Source Dwarven applies to the shooter's actual height at firing time.
pub(crate) fn projectile_stats_at(stack: &ItemStack, y: f32, max_height: i32) -> Option<(f32, f32)> {
    projectile_stats_at_airborne(stack, y, max_height, false)
}

pub(crate) fn projectile_stats_at_airborne(stack: &ItemStack, y: f32, max_height: i32, airborne: bool) -> Option<(f32, f32)> {
    let (mut velocity, spread) = projectile_stats(stack)?;
    let custom = stack.data()?.other("minecraft:custom_data")?;
    if airborne && matches!(custom.get("tic_airborne"), Some(Tag::Int(level)) if *level > 0) {
        if let Some(multiplier) = stat(stack, "tic_airborne_velocity_multiplier").filter(|value| value.is_finite()) {
            velocity += 0.1 * multiplier;
        }
    }
    let level = match custom.get("tic_dwarven") { Some(Tag::Int(value)) => *value, _ => 0 };
    if let Some(multiplier) = stat(stack, "tic_dwarven_velocity_multiplier").filter(|value| value.is_finite()) {
        if level > 0 && y.is_finite() {
            velocity = super::tconstruct_source_dwarven::boost(y, max_height, level, velocity, 0.05 * multiplier);
        }
    }
    // Overwield's source priority 75 runs after Dwarven's priority 85.
    let level = match custom.get("tic_overwield_ranged") { Some(Tag::Int(n)) => (*n).max(0), _ => 0 };
    let amount = match custom.get("tic_overslime") { Some(Tag::Int(n)) => (*n).max(0), _ => 0 };
    if let Some(multiplier) = stat(stack, "tic_overwield_velocity_multiplier").filter(|value| value.is_finite()) {
        velocity += level.min(amount) as f32 * 0.06 * multiplier;
    }
    Some((velocity.clamp(0.0, 1024.0), spread))
}

pub(super) fn airborne_conditions(on_ground: bool, in_water: bool, riding: bool, climbable: bool) -> bool {
    !on_ground && !in_water && !riding && !climbable
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

pub(crate) fn ambush_level(stack: &ItemStack) -> i32 {
    match stack.data().and_then(|data| data.other("minecraft:custom_data"))
        .and_then(|custom| custom.get("tic_ambush_projectile")) {
        Some(Tag::Int(level)) => (*level).max(0), _ => 0,
    }
}

/// Source full-health predicate uses ceil(current health), not exact equality.
fn full_health(health: f32, maximum: f32) -> bool {
    health.is_finite() && maximum.is_finite() && maximum > 0.0 && health.ceil() >= maximum
}

/// Conditional projectile damage evaluated for the acting shooter at release.
/// Missing/invalid maximum health skips Ambush rather than guessing a maximum.
pub(crate) fn projectile_bonus_at(stack: &ItemStack, health: Option<f32>, maximum: Option<f32>) -> f64 {
    let base = projectile_bonus(stack);
    let Some(health) = health.filter(|value| value.is_finite()) else { return base; };
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return base; };
    let Some(multiplier) = stat(stack, "tic_raging_projectile_multiplier").filter(|value| value.is_finite()) else { return base; };
    let raging = match custom.get("tic_raging") { Some(Tag::Int(level)) => (*level).max(0), _ => 0 };
    let ambush = ambush_level(stack);
    let raging_bonus = ((11.0 - health) / 10.0).clamp(0.0, 1.0) * raging as f32 * multiplier;
    let ambush_bonus = if maximum.is_some_and(|maximum| full_health(health, maximum)) {
        0.5 * ambush as f32 * multiplier
    } else { 0.0 };
    // Equal-priority flat effects retain material insertion order. Old items
    // without the saved order keep the prior Raging-first fallback.
    let ambush_first = match custom.get("tic_trait_order") {
        Some(Tag::List(ids)) => {
            let position = |name: &str| ids.iter().position(|id| matches!(id, Tag::String(id) if id.to_string_lossy() == name));
            matches!((position("tconstruct:ambush"), position("tconstruct:raging")), (Some(a), Some(r)) if a < r)
        }
        _ => false,
    };
    // Conditional modules operate on the original float stat, not a narrowed
    // native-arrow damage delta. Subtracting two before narrowing loses bits.
    let mut value = stat(stack, "tic_projectile_damage").filter(|value| value.is_finite())
        .map(|value| value.max(0.0)).unwrap_or((base + 2.0) as f32);
    if ambush_first { value += ambush_bonus; value += raging_bonus; }
    else { value += raging_bonus; value += ambush_bonus; }
    f64::from(value.clamp(0.0, 1024.0)) - 2.0
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
    fn maintained_tracks_wear_and_precedes_dwarven_penalty() {
        for id in ["tconstruct:longbow", "tconstruct:crossbow"] {
            let mut stack = bow(id, 1.0, 1.0);
            stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
                ("tic_velocity", Tag::Float(1.0f32.to_bits())),
                ("tic_accuracy", Tag::Float(1.0f32.to_bits())),
                ("tic_max_damage", Tag::Int(100)),
                ("tic_maintained", Tag::Int(2)),
                ("tic_maintained_velocity_multiplier", Tag::Float(1.5f32.to_bits())),
                ("tic_dwarven", Tag::Int(1)),
                ("tic_dwarven_velocity_multiplier", Tag::Float(1.5f32.to_bits())),
            ])));
            for (damage, expected) in [(0, 1.24), (25, 1.12), (50, 1.0), (99, 1.0), (100, 1.0), (101, 1.0)] {
                stack.damage = damage;
                assert!((projectile_stats(&stack).unwrap().0 - expected).abs() < 0.00001);
                assert!((projectile_stats_at(&stack, 256.0, 320).unwrap().0 - expected * 0.25).abs() < 0.00001);
            }
            stack.damage = 0; // A repaired tool regains the effect without rebuilding.
            assert!((projectile_stats_at(&stack, 0.0, 320).unwrap().0 - 1.315).abs() < 0.00001);
        }
    }
    #[test]
    fn airborne_predicate_excludes_ground_water_riding_and_climbing() {
        assert!(airborne_conditions(false, false, false, false));
        assert!(!airborne_conditions(true, false, false, false));
        assert!(!airborne_conditions(false, true, false, false));
        assert!(!airborne_conditions(false, false, true, false));
        assert!(!airborne_conditions(false, false, false, true));
    }
    #[test]
    fn airborne_bonus_precedes_dwarven_penalty() {
        let mut stack = bow("tconstruct:longbow", 1.0, 1.0);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_velocity", Tag::Float(1.0f32.to_bits())),
            ("tic_accuracy", Tag::Float(1.0f32.to_bits())),
            ("tic_airborne", Tag::Int(2)),
            ("tic_airborne_velocity_multiplier", Tag::Float(2.0f32.to_bits())),
            ("tic_dwarven", Tag::Int(1)),
            ("tic_dwarven_velocity_multiplier", Tag::Float(2.0f32.to_bits())),
        ])));
        assert!((projectile_stats_at_airborne(&stack, 256.0, 320, true).unwrap().0 - 0.3).abs() < 1e-6);
        assert!((projectile_stats_at_airborne(&stack, 256.0, 320, false).unwrap().0 - 0.25).abs() < 1e-6);
    }
    #[test]
    fn high_altitude_penalizes_airborne_before_overwield() {
        for id in ["tconstruct:longbow", "tconstruct:crossbow"] {
            let mut stack = bow(id, 1.0, 1.0);
            stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
                ("tic_velocity", Tag::Float(1.0f32.to_bits())),
                ("tic_accuracy", Tag::Float(1.0f32.to_bits())),
                ("tic_airborne", Tag::Int(1)),
                ("tic_airborne_velocity_multiplier", Tag::Float(2.0f32.to_bits())),
                ("tic_dwarven", Tag::Int(1)),
                ("tic_dwarven_velocity_multiplier", Tag::Float(2.0f32.to_bits())),
                ("tic_overwield_ranged", Tag::Int(1)), ("tic_overslime", Tag::Int(1)),
                ("tic_overwield_velocity_multiplier", Tag::Float(2.0f32.to_bits())),
            ])));
            // (1 + 0.1 * 2) * 0.25 + 0.06 * 2
            assert!((projectile_stats_at_airborne(&stack, 256.0, 320, true).unwrap().0 - 0.42).abs() < 1e-6);
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

#[cfg(test)]
mod accuracy_boundary_tests {
    use super::*;
    #[test]
    fn finite_positive_accuracy_is_clamped_before_reciprocal_spread() {
        for id in ["tconstruct:longbow","tconstruct:crossbow"] {
            for accuracy in [f32::from_bits(1),0.05,0.1,f32::from_bits(0.1f32.to_bits()+1),0.75,1.0,2.0,f32::MAX] {
                let mut stack=ItemStack::new(id,1);
                stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
                    ("tic_velocity",Tag::Float(1.0f32.to_bits())),("tic_accuracy",Tag::Float(accuracy.to_bits())),
                ])));
                let expected=3.0f32*(1.0f32/accuracy.clamp(0.1,1.0)-1.0);
                assert_eq!(projectile_stats(&stack),Some((1.0,expected)));
                assert_eq!(projectile_stats_at_airborne(&stack,64.0,320,true),Some((1.0,expected)));
                assert_eq!(stat(&stack,"tic_accuracy"),Some(accuracy));
            }
            for accuracy in [0.0,-1.0,f32::NAN,f32::INFINITY,f32::NEG_INFINITY] {
                let mut stack=ItemStack::new(id,1);
                stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
                    ("tic_velocity",Tag::Float(1.0f32.to_bits())),("tic_accuracy",Tag::Float(accuracy.to_bits())),
                ])));
                assert_eq!(projectile_stats(&stack),None);
            }
        }
    }
}

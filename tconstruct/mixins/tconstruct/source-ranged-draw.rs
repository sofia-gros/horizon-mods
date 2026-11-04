use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

const SNAPSHOT: &str = "tic_draw_ticks";

fn ticks_for_speed(speed: f32) -> u32 {
    // Java getDrawtime narrows Math.ceil to signed int before host conversion.
    // Clamp after narrowing: i32::MAX as f32 rounds up to 2^31.
    ((20.0 / speed).ceil() as i32).max(1) as u32
}

pub(crate) fn momentum_level(stack: &ItemStack) -> i32 {
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|c| c.get("tic_momentum")) {
        Some(Tag::Int(level)) => (*level).max(0),
        _ => 0,
    }
}

// Both hooks have priority 75. Source merges upgrades before traits and keeps
// insertion order. Never invent an order for legacy or malformed snapshots.
fn momentum_first(custom: &Tag) -> Option<bool> {
    let mut ids = Vec::new();
    for key in ["tic_upgrade_order", "tic_trait_order"] {
        match custom.get(key) {
            Some(Tag::List(values)) => for value in values {
                let Tag::String(id) = value else { return None; };
                let id = id.to_string_lossy();
                if !ids.contains(&id) { ids.push(id); }
            },
            None => {},
            _ => return None,
        }
    }
    let momentum = ids.iter().position(|id| id == "tconstruct:momentum")?;
    let overwield = ids.iter().position(|id| id == "tconstruct:overwield")?;
    Some(momentum < overwield)
}

fn conditional_speed(speed: f32, bonus: f32, factor: f32, first: Option<bool>) -> f32 {
    let speed = if bonus == 0.0 { speed * factor }
        else if factor == 1.0 { speed + bonus }
        else { match first {
            Some(true) => speed * factor + bonus,
            Some(false) => (speed + bonus) * factor,
            None => speed + bonus, // Unknown tie order: retain Overwield only.
        }};
    speed.clamp(0.0, 1024.0)
}

// Evaluate only when a new use starts. In particular, never reuse SNAPSHOT
// here: a previous use, a save, or an inventory copy may still contain it.
pub(crate) fn fresh_draw_ticks(stack: &ItemStack, effect_level: i32) -> u32 {
    let custom = stack.data().and_then(|d| d.other("minecraft:custom_data"));
    let speed = match custom.and_then(|c| c.get("tic_draw_speed")) {
        Some(Tag::Float(bits)) => f32::from_bits(*bits),
        _ => 1.0,
    };
    let bonus = custom.and_then(|custom| {
        let level = match custom.get("tic_overwield_ranged") { Some(Tag::Int(n)) => (*n).max(0), _ => 0 };
        let amount = match custom.get("tic_overslime") { Some(Tag::Int(n)) => (*n).max(0), _ => 0 };
        let multiplier = match custom.get("tic_overwield_draw_multiplier") { Some(Tag::Float(bits)) => f32::from_bits(*bits), _ => return None };
        multiplier.is_finite().then_some(level.min(amount) as f32 * 0.06 * multiplier)
    }).unwrap_or(0.0);
    let factor = 1.0 + effect_level.max(0) as f32 * momentum_level(stack) as f32 / 40.0;
    ticks_for_speed(conditional_speed(speed, bonus, factor, custom.and_then(momentum_first)))
}

pub(crate) fn draw_ticks(stack: &ItemStack) -> u32 {
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|c| c.get(SNAPSHOT)) {
        Some(Tag::Int(ticks)) if *ticks > 0 => *ticks as u32,
        _ => fresh_draw_ticks(stack, 0),
    }
}

pub(crate) fn snapshot(stack: &mut ItemStack, effect_level: i32) {
    let ticks = fresh_draw_ticks(stack, effect_level);
    let mut custom = stack.data().and_then(|d| d.other("minecraft:custom_data")).cloned()
        .unwrap_or_else(|| Tag::Compound(Default::default()));
    let Tag::Compound(fields) = &mut custom else { return; };
    fields.insert(SNAPSHOT.into(), Tag::Int(ticks as i32));
    stack.data_mut().set_other("minecraft:custom_data", Some(custom));
}

pub(crate) fn clear_snapshot(stack: &mut ItemStack) {
    let Some(custom) = stack.data().and_then(|d| d.other("minecraft:custom_data")) else { return; };
    if custom.get(SNAPSHOT).is_none() { return; }
    let mut custom = custom.clone();
    let Tag::Compound(fields) = &mut custom else { return; };
    fields.remove(&SNAPSHOT.into());
    stack.data_mut().set_other("minecraft:custom_data", Some(custom));
}

// Host multishot marks free extra arrows intangible. That flag changes pickup,
// not their source ammo identity. Unknown/mixed payloads do not earn Momentum.
pub(crate) fn arrow_primary(charged: &[ItemStack]) -> Option<usize> {
    let normalized = |ammo: &ItemStack| {
        let mut ammo = ammo.clone();
        ammo.data_mut().set_other("minecraft:intangible_projectile", None);
        ammo.tidy();
        ammo
    };
    let first = charged.first()?;
    if !matches!(&*first.item, "minecraft:arrow" | "minecraft:tipped_arrow" | "minecraft:spectral_arrow") { return None; }
    let first = normalized(first);
    charged.iter().all(|ammo| normalized(ammo).same_item_same_components(&first)).then_some(charged.len() / 2)
}

pub(crate) fn earns_momentum(qualified: bool, primary: bool, bow: bool, power: f32, native_critical: bool) -> bool {
    // The existing host bridge may rescale an ArrowLoose event's charge. Do not
    // award a full-charge bonus if either the source or native arrow is partial.
    qualified && primary && native_critical && (!bow || power >= 3.0)
}

pub(crate) fn vanilla_charge_ticks(elapsed: u32, draw_ticks: u32) -> u32 {
    (elapsed as f32 * 20.0 / draw_ticks as f32).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bow() -> ItemStack {
        let mut stack = ItemStack::new("tconstruct:longbow", 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_draw_speed", Tag::Float(1.0f32.to_bits())),
            ("tic_momentum", Tag::Int(1)),
            ("fixture:keep", Tag::Int(42)),
        ])));
        stack
    }

    #[test]
    fn draw_time_rounds_up_and_charge_scales_to_vanilla_ticks() {
        assert_eq!(ticks_for_speed(1.0), 20);
        assert_eq!(ticks_for_speed(1.5), 14);
        assert_eq!(ticks_for_speed(0.75), 27);
        assert_eq!(vanilla_charge_ticks(14, 14), 20);
        assert_eq!(vanilla_charge_ticks(7, 14), 10);
        assert_eq!(vanilla_charge_ticks(0, 14), 0);
    }

    #[test]
    fn active_snapshot_survives_effect_change_but_not_a_new_use() {
        let mut stack = bow();
        snapshot(&mut stack, 10);
        assert_eq!(draw_ticks(&stack), 16);
        assert_eq!(fresh_draw_ticks(&stack, 0), 20);
        assert_eq!(draw_ticks(&stack), 16);
        snapshot(&mut stack, 0);
        assert_eq!(draw_ticks(&stack), 20);
        clear_snapshot(&mut stack);
        assert_eq!(draw_ticks(&stack), 20);
        assert_eq!(stack.data().unwrap().other("minecraft:custom_data").unwrap().get("fixture:keep"), Some(&Tag::Int(42)));
    }

    #[test]
    fn equal_priority_modifiers_keep_explicit_order_or_skip_momentum() {
        assert_eq!(conditional_speed(1.0, 0.06, 1.25, Some(true)), 1.31);
        assert_eq!(conditional_speed(1.0, 0.06, 1.25, Some(false)), 1.3249999);
        assert_eq!(conditional_speed(1.0, 0.06, 1.25, None), 1.06);
        let order = |key, ids: &[&str]| Tag::compound([(key, Tag::List(ids.iter().map(|id| Tag::String((*id).into())).collect()))]);
        assert_eq!(momentum_first(&order("tic_trait_order", &["tconstruct:momentum", "tconstruct:overwield"])), Some(true));
        assert_eq!(momentum_first(&order("tic_trait_order", &["tconstruct:overwield", "tconstruct:momentum"])), Some(false));
        assert_eq!(momentum_first(&order("tic_trait_order", &["tconstruct:momentum"])), None);
    }

    #[test]
    fn multishot_primary_ignores_pickup_flag_but_rejects_mixed_ammo() {
        let arrow = ItemStack::new("minecraft:arrow", 1);
        let mut free = arrow.clone();
        free.data_mut().set_other("minecraft:intangible_projectile", Some(Tag::Compound(Default::default())));
        assert_eq!(arrow_primary(&[arrow.clone(), free.clone(), free]), Some(1));
        assert_eq!(arrow_primary(&[arrow, ItemStack::new("minecraft:spectral_arrow", 1)]), None);
        assert_eq!(arrow_primary(&[ItemStack::new("minecraft:firework_rocket", 1)]), None);
        assert_eq!(arrow_primary(&[]), None);
    }

    #[test]
    fn only_qualified_primary_full_launches_earn_stacks() {
        assert!(earns_momentum(true, true, true, 3.0, true));
        assert!(earns_momentum(true, true, false, 3.0, true));
        for args in [(false, true, true, 3.0, true), (true, false, true, 3.0, true),
            (true, true, true, 2.99, true), (true, true, true, 3.0, false)] {
            assert!(!earns_momentum(args.0, args.1, args.2, args.3, args.4));
        }
    }
}

#[cfg(test)]
mod saturation_tests {
    use super::*;
    #[test]
    fn signed_draw_time_saturation_precedes_unsigned_host_conversion() {
        let boundary = 20.0f32 / 2147483648.0f32;
        for speed in [0.0, -0.0, f32::from_bits(1), f32::MIN_POSITIVE, boundary] {
            assert_eq!(ticks_for_speed(speed), if speed.is_sign_negative() { 1 } else { i32::MAX as u32 });
        }
        assert_eq!(ticks_for_speed(f32::from_bits(boundary.to_bits()+1)), 2147483392);
        for speed in [f32::NAN, f32::INFINITY, -1.0, f32::NEG_INFINITY] {
            assert_eq!(ticks_for_speed(speed), 1); // Existing host minimum, not source NaN=0.
        }
        assert_eq!(ticks_for_speed(1024.0), 1);
    }
}

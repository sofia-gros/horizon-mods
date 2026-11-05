// EnchantmentModule.Protection adds floor(effective level) without adding its
// vanilla damage protection. ProtectionEnchantment uses the highest armor level.
pub fn enchantment_level(traits: i32, raw: i32) -> Option<i32> {
    if traits < 0 || !(0..=5 * 32767).contains(&raw) { return None; }
    let whole = traits.checked_add(raw / 5)?;
    let remainder = raw % 5;
    let level = if remainder == 0 { whole as f32 }
        else { whole.checked_add(1)? as f32 + (remainder - 5) as f32 / 5.0 };
    Some(level as i32)
}

/// Source multiplies in f32 then floors the reduction, not the final duration.
pub fn duration(ticks: i32, level: i32) -> Option<i32> {
    if ticks < 0 || level <= 0 { return None; }
    let reduction = ((ticks as f32 * level as f32) * 0.15f32).floor();
    if !reduction.is_finite() || reduction >= i32::MAX as f32 { return None; }
    ticks.checked_sub(reduction as i32)
}

/// The native scaled value is already computed. A different native result
/// means another attribute contribution is active, so preserve that result.
pub fn explicit_duration(value: i32, ticks: i32, context: Option<(i32, f64)>) -> i32 {
    let Some((level, native_factor)) = context else { return value; };
    if !native_factor.is_finite()
        || value != (f64::from(ticks) * native_factor).ceil() as i32 { return value; }
    duration(ticks, level).unwrap_or(value)
}

/// Normalize only the call's local multiplier. The host evaluates
/// ceil(160 * multiplier); placing the product halfway inside the desired
/// integer's interval preserves the exact source duration despite f64 division.
/// No saved/live attribute changes, extra random draw or second ignition occurs.
pub fn contact_multiplier(value: f64, context: Option<(i32, f64)>) -> f64 {
    let Some((level, native_factor)) = context else { return value; };
    if !value.is_finite() || value != native_factor { return value; }
    duration(160, level).map_or(value, |ticks| (f64::from(ticks) - 0.5) / 160.0)
}

pub(super) fn armor_slot(item: &str) -> Option<usize> {
    match item {
        "tconstruct:travelers_helmet" | "tconstruct:plate_helmet" | "tconstruct:slime_helmet" => Some(0),
        "tconstruct:travelers_chestplate" | "tconstruct:plate_chestplate" | "tconstruct:slimy_chestplate" | "tconstruct:slime_wings" => Some(1),
        "tconstruct:travelers_leggings" | "tconstruct:plate_leggings" | "tconstruct:slime_leggings" => Some(2),
        "tconstruct:travelers_boots" | "tconstruct:plate_boots" | "tconstruct:slime_boots" => Some(3),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enchantment_uses_completed_effective_levels_not_started_levels() {
        assert_eq!(enchantment_level(0, 4), Some(0));
        assert_eq!(enchantment_level(0, 5), Some(1));
        assert_eq!(enchantment_level(2, 7), Some(3));
        assert_eq!(enchantment_level(-1, 0), None);
        assert_eq!(enchantment_level(0, 163836), None);
        assert_eq!(enchantment_level(i32::MAX, 1), None);
    }
    #[test]
    fn original_float_reduction_and_unlimited_level_are_preserved() {
        assert_eq!(duration(300, 1), Some(255));
        assert_eq!(duration(160, 2), Some(112));
        assert_eq!(duration(1, 1), Some(1));
        assert_eq!(duration(300, 7), Some(-15));
        assert_eq!(duration(-1, 1), None);
        assert_eq!(duration(100, 0), None);
    }
}

#[cfg(test)]
mod contact_tests {
    use super::*;
    #[test]
    fn normalized_multiplier_reproduces_source_integer_for_supported_levels() {
        for level in 1..=32767 {
            let factor = contact_multiplier(1.0, Some((level, 1.0)));
            assert_eq!((160.0 * factor).ceil() as i32, duration(160, level).unwrap());
        }
    }
    #[test]
    fn missing_context_or_changed_native_attribute_keeps_native_behavior() {
        assert_eq!(contact_multiplier(0.8, None), 0.8);
        assert_eq!(contact_multiplier(0.8, Some((2, 1.0))), 0.8);
        assert_eq!(contact_multiplier(0.8, Some((0, 0.8))), 0.8);
        assert!(contact_multiplier(f64::NAN, Some((2, 1.0))).is_nan());
        assert_eq!(explicit_duration(300, 300, Some((1, 1.0))), 255);
        assert_eq!(explicit_duration(240, 300, Some((1, 1.0))), 240);
        assert_eq!(explicit_duration(300, 300, None), 300);
    }
}

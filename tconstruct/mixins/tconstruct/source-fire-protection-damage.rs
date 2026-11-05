/// Intact source tools provide 3 EPF per effective level. Their virtual
/// vanilla Fire Protection is canceled by EnchantmentModule.Protection, so it
/// must not be added a second time. Broken worn armor retains only the virtual
/// vanilla enchantment: the source equipment iterator skips both modifier hooks.
pub(super) fn points(pieces: &[(f32, bool, bool)]) -> Option<(f32, f32)> {
    let mut source = 0.0f32;
    let mut broken_vanilla = 0.0f32;
    for (level, broken, worn) in pieces {
        if !level.is_finite() || *level < 0.0 { return None; }
        if *broken {
            if *worn { broken_vanilla += 2.0 * level.floor(); }
        } else { source += 3.0 * level; }
    }
    (source.is_finite() && broken_vanilla.is_finite()).then_some((source, broken_vanilla))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intact_levels_are_fractional_and_not_double_counted_as_enchantments() {
        assert_eq!(points(&[(1.0,false,true)]), Some((3.0,0.0)));
        assert_eq!(points(&[(0.4,false,true),(2.0,false,false)]), Some((3.0 * 0.4 + 6.0,0.0)));
        assert_eq!(points(&[(f32::NAN,false,true)]), None);
    }
    #[test]
    fn broken_armor_keeps_only_integer_virtual_fire_enchantment() {
        assert_eq!(points(&[(1.4,true,true),(2.0,true,false)]), Some((0.0,2.0)));
        assert_eq!(points(&[(0.4,true,true)]), Some((0.0,0.0)));
    }
}

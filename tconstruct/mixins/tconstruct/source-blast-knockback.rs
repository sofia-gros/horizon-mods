/// The source vanilla secondary effect uses the highest armor enchantment
/// level, not the newer native sum of per-piece resistance attributes.
pub(super) fn replacement(
    native: f64, pieces: &[(u32, i32)], native_amounts: &[f64],
) -> Option<f64> {
    if !native.is_finite() || native_amounts.iter().any(|n| !n.is_finite() || *n < 0.0) {
        return None;
    }
    let baseline = native_amounts.iter().fold(0.0f64, |value, amount| value + amount).clamp(0.0, 1.0);
    if native != baseline || !pieces.iter().any(|(_, source)| *source > 0) { return None; }
    let maximum = pieces.iter().try_fold(0i32, |maximum, (existing, source)| {
        if *source < 0 { return None; }
        Some(maximum.max(i32::try_from(*existing).ok()?.checked_add(*source)?))
    })?;
    Some((f64::from(maximum) * 0.15).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maximum_combines_each_pieces_native_enchantment_and_modifier() {
        assert_eq!(replacement(0.0, &[(0, 2), (0, 2)], &[]), Some(0.3));
        assert_eq!(replacement(0.0, &[(0, 7)], &[]), Some(1.0));
        let amount = f64::from(0.15f32);
        assert_eq!(replacement(amount, &[(1, 1), (0, 1)], &[amount]), Some(0.3));
        assert_eq!(replacement(amount, &[(1, 0), (0, 1)], &[amount]), Some(0.15));
    }
    #[test]
    fn source_maximum_replaces_native_sum_only_with_a_source_contribution() {
        let amount = f64::from(0.6f32);
        assert_eq!(replacement(1.0, &[(4, 0), (4, 1)], &[amount, amount]), Some(0.75));
        assert_eq!(replacement(1.0, &[(4, 0), (4, 0)], &[amount, amount]), None);
        assert_eq!(replacement(0.5, &[(0, 1)], &[]), None);
        assert_eq!(replacement(f64::NAN, &[(0, 1)], &[]), None);
        assert_eq!(replacement(0.0, &[(0, -1), (0, 1)], &[]), None);
        assert_eq!(replacement(0.0, &[(u32::MAX, 1)], &[]), None);
    }
}

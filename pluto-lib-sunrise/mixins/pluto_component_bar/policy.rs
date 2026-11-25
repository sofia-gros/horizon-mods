/// Return the host damage-bar inputs only when a stack supplies a valid maximum
/// and vanilla did not choose an unbreakable or bundle display.
pub fn bar_input(
    max: Option<i32>,
    damage: u32,
    unbreakable: bool,
    bundle: bool,
) -> Option<(i32, i32)> {
    let max = max.filter(|max| *max > 0)?;
    if unbreakable || bundle {
        return None;
    }
    Some((damage as i32, max))
}

#[cfg(test)]
mod tests {
    use super::bar_input;
    #[test]
    fn component_only_and_preserves_special_displays() {
        assert_eq!(bar_input(None, 1, false, false), None);
        assert_eq!(bar_input(Some(0), 1, false, false), None);
        assert_eq!(bar_input(Some(-1), 1, false, false), None);
        assert_eq!(bar_input(Some(60), 1, true, false), None);
        assert_eq!(bar_input(Some(60), 1, false, true), None);
        assert_eq!(bar_input(Some(60), 1, false, false), Some((1, 60)));
        assert_eq!(bar_input(Some(60), 0, false, false), Some((0, 60)));
    }
}

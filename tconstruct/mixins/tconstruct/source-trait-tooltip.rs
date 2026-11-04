// Only the two stock definitions whose display behavior is verified here.
// Refuse unknown definitions, invalid levels, and an overlapping upgrade.
#[derive(Clone, Copy)]
pub(super) enum DisplayFlag { Missing, Byte(i8), Malformed }
pub(super) fn display_flag_valid(flag: DisplayFlag, shift: bool, control: bool) -> bool {
    match flag {
        DisplayFlag::Byte(1) => true,
        DisplayFlag::Missing | DisplayFlag::Byte(0) => !shift && !control,
        _ => false,
    }
}
pub(super) fn visible(id: &str, level: i32, known_stock: bool, upgrade: Option<i32>) -> bool {
    matches!(id, "tconstruct:jagged" | "tconstruct:stonebound")
        && (1..=10).contains(&level) && known_stock && upgrade == Some(0)
}
pub(super) fn roman(level: i32) -> Option<&'static str> {
    ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"].get(usize::try_from(level.checked_sub(1)?).ok()?).copied()
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn source_display_flags_are_validated() {
        for (shift, control) in [(false, false), (true, false), (false, true), (true, true)] {
            assert!(display_flag_valid(DisplayFlag::Byte(1), shift, control));
            assert_eq!(display_flag_valid(DisplayFlag::Missing, shift, control), !shift && !control);
            assert_eq!(display_flag_valid(DisplayFlag::Byte(0), shift, control), !shift && !control);
            assert!(!display_flag_valid(DisplayFlag::Byte(2), shift, control));
            assert!(!display_flag_valid(DisplayFlag::Malformed, shift, control));
        }
    }
    #[test] fn limits_display_to_verified_nonoverlapping_stock() {
        for id in ["tconstruct:jagged", "tconstruct:stonebound"] {
            assert!(visible(id, 1, true, Some(0)));
            assert!(visible(id, 10, true, Some(0)));
            assert!(!visible(id, 11, true, Some(0)));
            assert!(!visible(id, 1, false, Some(0)));
            assert!(!visible(id, 1, true, None));
            assert!(!visible(id, 1, true, Some(1)));
        }
        assert!(!visible("tconstruct:pocket", 1, true, Some(0)));
        assert_eq!(roman(1), Some("I")); assert_eq!(roman(10), Some("X")); assert_eq!(roman(0), None);
    }
}

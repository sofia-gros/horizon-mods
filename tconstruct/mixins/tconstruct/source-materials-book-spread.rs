// usize::MAX represents the cover without adding a synthetic wire page.
pub(crate) const COVER: usize = usize::MAX;
pub(crate) fn turn(page: usize, count: usize, forward: bool) -> usize {
    if page == COVER { return if forward && count > 0 { 0 } else { COVER }; }
    let base = (page / 2) * 2;
    if forward { if base.saturating_add(2) < count { base + 2 } else { base } }
    else if base == 0 { COVER } else { base - 2 }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cover_and_spreads_turn_in_both_directions() {
        assert_eq!(turn(COVER, 5, false), COVER);
        assert_eq!(turn(COVER, 5, true), 0);
        assert_eq!(turn(0, 5, false), COVER);
        assert_eq!(turn(1, 5, true), 2);
        assert_eq!(turn(3, 5, false), 0);
        assert_eq!(turn(4, 5, true), 4);
        assert_eq!(turn(COVER, 0, true), COVER);
    }
}

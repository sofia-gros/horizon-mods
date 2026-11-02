// usize::MAX represents the cover without adding a synthetic wire page.
pub(crate) const COVER: usize = usize::MAX;
pub(crate) fn turn(page: usize, count: usize, forward: bool) -> usize {
    if page == COVER { return if forward && count > 0 { 0 } else { COVER }; }
    let base = (page / 2) * 2;
    if forward { if base.saturating_add(2) < count { base + 2 } else { base } }
    else if base == 0 { COVER } else { base - 2 }
}
pub(crate) fn remember(history: &mut Option<usize>, current: usize, target: i32, count: usize) {
    if target > 0 && (target as usize) <= count && (target as usize - 1) != current { *history = Some(current); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn link_history_remembers_only_valid_page_changes() {
        let mut history = None;
        remember(&mut history, 4, 9, 12);
        assert_eq!(history.take(), Some(4));
        remember(&mut history, 4, 5, 12);
        remember(&mut history, 4, 13, 12);
        assert_eq!(history, None);
    }
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

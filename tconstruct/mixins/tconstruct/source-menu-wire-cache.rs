/// Keep our framed menu data separate from Horizon's vanilla data-slot cache.
pub(super) fn changed(old: Option<&(i16, Vec<i16>)>, selection: i16, frame: &[i16]) -> (bool, bool) {
    (old.is_none_or(|(previous, _)| *previous != selection), old.is_none_or(|(_, previous)| previous != frame))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_and_frame_changes_are_independent() {
        assert_eq!(changed(None, -1, &[1, 2]), (true, true));
        let old = (-1, vec![1, 2]);
        assert_eq!(changed(Some(&old), -1, &[1, 2]), (false, false));
        assert_eq!(changed(Some(&old), 0, &[1, 2]), (true, false));
        assert_eq!(changed(Some(&old), -1, &[1, 3]), (false, true));
    }
}

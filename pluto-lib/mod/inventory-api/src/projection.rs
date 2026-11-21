//! Read-only projection of caller-owned slots into an explicitly mapped view.

/// Clone one output cell per mapping entry, in mapping order.
///
/// `None` denotes an unmapped cell. An out-of-bounds source index or an empty
/// source cell also produces `None`. Repeated indices clone the same source
/// cell again; they are not deduplicated. The source is borrowed and unchanged.
/// Cloned values retain their type's normal `Clone` semantics.
///
/// Callers own layout, scrolling, padding, reserved cells and visibility rules.
/// This display projection does not validate a transaction or authorize a
/// write. Keep the mapping separately and reject virtual/padding destinations
/// before committing through the authoritative storage path.
pub fn project_slots<T: Clone>(
    slots: &[Option<T>],
    mapping: impl IntoIterator<Item = Option<usize>>,
) -> Vec<Option<T>> {
    mapping.into_iter().map(|index| {
        index.and_then(|index| slots.get(index)).cloned().flatten()
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_order_duplicates_and_padding_are_explicit() {
        let slots = [Some("first"), None, Some("last")];
        assert_eq!(project_slots(&slots, [Some(2), None, Some(0), Some(2), Some(1)]),
            vec![Some("last"), None, Some("first"), Some("last"), None]);
        assert_eq!(slots, [Some("first"), None, Some("last")]);
    }

    #[test]
    fn unavailable_indexes_do_not_map_to_another_slot() {
        assert_eq!(project_slots(&[Some(7)], [Some(1), Some(usize::MAX), Some(0)]),
            vec![None, None, Some(7)]);
        assert_eq!(project_slots::<u8>(&[], [Some(0), None]), vec![None, None]);
        assert!(project_slots(&[Some(7)], []).is_empty());
    }

    #[test]
    fn full_values_are_cloned_without_interpreting_their_fields() {
        let slots = [Some(vec![1, 2, 3])];
        let mut view = project_slots(&slots, [Some(0), Some(0)]);
        view[0].as_mut().unwrap().push(4);
        assert_eq!(slots[0], Some(vec![1, 2, 3]));
        assert_eq!(view[1], Some(vec![1, 2, 3]));
    }

    #[test]
    fn mapping_is_consumed_once_in_order() {
        let mut calls = Vec::new();
        let view = project_slots(&[Some(9)], (0..3).map(|index| {
            calls.push(index);
            (index != 1).then_some(0)
        }));
        assert_eq!(calls, vec![0, 1, 2]);
        assert_eq!(view, vec![Some(9), None, Some(9)]);
    }
}

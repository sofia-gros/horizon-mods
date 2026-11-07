//! Build partitioned storage proposals before the caller commits any of them.
use std::collections::{btree_map::Entry, BTreeMap};

/// Stage changes in their supplied order, loading each partition only once.
///
/// `load_copy` must return an independent, caller-owned proposal value, not an
/// alias through which `apply_to_copy` can mutate live storage. Missing input or
/// a rejected change aborts and drops all staged values. No later change runs.
/// Successful proposals are returned in key order; an empty input succeeds
/// without invoking callbacks.
///
/// This helper does not publish, lock, persist or roll back authoritative state.
/// Callback side effects outside the supplied proposal values are the caller's
/// responsibility and cannot be undone here. Revalidate and commit proposals in
/// the caller's authoritative transaction, under its schema and lifecycle rules.
pub fn stage_updates<K: Ord, V, C>(
    changes: impl IntoIterator<Item = C>,
    mut partition_key: impl FnMut(&C) -> K,
    mut load_copy: impl FnMut(&K) -> Option<V>,
    mut apply_to_copy: impl FnMut(&mut V, C) -> bool,
) -> Option<BTreeMap<K, V>> {
    let mut staged = BTreeMap::new();
    for change in changes {
        let key = partition_key(&change);
        let proposal = match staged.entry(key) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let value = load_copy(entry.key())?;
                entry.insert(value)
            }
        };
        if !apply_to_copy(proposal, change) { return None; }
    }
    Some(staged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn partitions_load_once_and_changes_keep_input_order() {
        let original = BTreeMap::from([(1, vec![10]), (3, vec![30])]);
        let loads = RefCell::new(Vec::new());
        let applications = RefCell::new(Vec::new());
        let staged = stage_updates([(3, 31), (1, 11), (3, 32)], |change| change.0,
            |key| { loads.borrow_mut().push(*key); original.get(key).cloned() },
            |value, (key, next)| { applications.borrow_mut().push((key, next)); value.push(next); true },
        ).unwrap();
        assert_eq!(*loads.borrow(), vec![3, 1]);
        assert_eq!(*applications.borrow(), vec![(3, 31), (1, 11), (3, 32)]);
        assert_eq!(staged.into_iter().collect::<Vec<_>>(), vec![(1, vec![10, 11]), (3, vec![30, 31, 32])]);
        assert_eq!(original, BTreeMap::from([(1, vec![10]), (3, vec![30])]));
    }

    #[test]
    fn missing_partition_discards_earlier_proposals_and_stops_callbacks() {
        let original = BTreeMap::from([(0, 4)]);
        let loads = RefCell::new(Vec::new()); let applied = RefCell::new(Vec::new());
        let staged = stage_updates([0, 1, 2], |key| *key,
            |key| { loads.borrow_mut().push(*key); original.get(key).copied() },
            |value, key| { *value += 1; applied.borrow_mut().push(key); true },
        );
        assert!(staged.is_none());
        assert_eq!(*loads.borrow(), vec![0, 1]); assert_eq!(*applied.borrow(), vec![0]);
        assert_eq!(original[&0], 4);
    }

    #[test]
    fn rejected_change_drops_partial_group_without_publishing() {
        let original = BTreeMap::from([(0, vec![1]), (1, vec![2])]);
        let applied = RefCell::new(Vec::new());
        let staged = stage_updates([(0, 3), (1, 4), (0, 5)], |change| change.0,
            |key| original.get(key).cloned(),
            |value, (key, next)| { value.push(next); applied.borrow_mut().push(key); key != 1 },
        );
        assert!(staged.is_none()); assert_eq!(*applied.borrow(), vec![0, 1]);
        assert_eq!(original, BTreeMap::from([(0, vec![1]), (1, vec![2])]));
    }

    #[test]
    fn empty_input_does_not_load_or_apply() {
        let staged = stage_updates::<u8, u8, u8>([], |_| panic!("partition"), |_| panic!("load"), |_, _| panic!("apply"));
        assert_eq!(staged, Some(BTreeMap::new()));
    }

    #[test]
    fn keys_and_proposals_need_not_be_clone() {
        #[derive(PartialEq, Eq, PartialOrd, Ord)] struct Key(u8);
        struct Proposal(u8);
        let staged = stage_updates([1, 2], |value| Key(*value), |key| Some(Proposal(key.0)),
            |proposal, _| { proposal.0 += 1; true }).unwrap();
        assert_eq!(staged.get(&Key(1)).unwrap().0, 2);
        assert_eq!(staged.get(&Key(2)).unwrap().0, 3);
    }
}

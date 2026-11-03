// Per-island marker decisions from IslandPiece at f4e77d4.
// The caller owns persistence, random draws and actual block/feature writes.

pub(crate) const MAX_TREES: u32 = 3;

/// Select a weighted entry from one already-drawn value in `[0, total)`.
pub(crate) fn weighted_index(weights: &[u32], draw: u32) -> Option<usize> {
    let total = weights.iter().try_fold(0u32, |sum, weight| sum.checked_add(*weight))?;
    if draw >= total { return None; }
    let mut remaining = draw;
    for (index, weight) in weights.iter().enumerate() {
        if remaining < *weight { return Some(index); }
        remaining -= *weight;
    }
    None
}

/// The count belongs to the island piece, not to an individual marker/chunk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TreeBudget { placed: u32 }

impl TreeBudget {
    pub(crate) fn from_saved(placed: u32) -> Option<Self> {
        (placed <= MAX_TREES).then_some(Self { placed })
    }

    pub(crate) fn placed(self) -> u32 { self.placed }

    /// Source draws its boolean only when a tree exists and capacity remains;
    /// a failed feature placement does not spend the island's tree budget.
    pub(crate) fn tree_marker(
        &mut self,
        has_tree: bool,
        draw: impl FnOnce() -> bool,
        place: impl FnOnce() -> bool,
    ) -> bool {
        if !has_tree || self.placed >= MAX_TREES || !draw() { return false; }
        if !place() { return false; }
        self.placed += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_source_choices_cover_exact_ranges() {
        assert_eq!((0..4).map(|n| weighted_index(&[1, 3], n)).collect::<Vec<_>>(),
                   [Some(0), Some(1), Some(1), Some(1)]);
        assert_eq!(weighted_index(&[7, 1], 6), Some(0));
        assert_eq!(weighted_index(&[7, 1], 7), Some(1));
        assert_eq!(weighted_index(&[7, 1], 8), None);
        assert_eq!(weighted_index(&[u32::MAX, 1], 0), None);
    }

    #[test]
    fn only_successful_trees_spend_shared_budget() {
        let mut budget = TreeBudget::default();
        assert!(!budget.tree_marker(false, || panic!("no tree: no random draw"), || true));
        assert!(!budget.tree_marker(true, || false, || panic!("failed draw: no placement")));
        assert!(!budget.tree_marker(true, || true, || false));
        for _ in 0..3 { assert!(budget.tree_marker(true, || true, || true)); }
        assert!(!budget.tree_marker(true, || panic!("full budget: no draw"), || true));
        assert_eq!(TreeBudget::from_saved(budget.placed()), Some(budget));
        assert!(TreeBudget::from_saved(4).is_none());
    }
}

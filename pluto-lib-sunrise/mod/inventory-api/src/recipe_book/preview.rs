//! Explicit read-only recipe selection and ghost presentation.
//!
//! A shown preview is not a placement permission or proof of craftability.
//! Consumer code still owns selection policy and any existing placement veto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookKind {
    Crafting,
    Furnace,
    BlastFurnace,
    Smoker,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryFacts {
    pub recipe_id: String,
    pub display_id: i32,
    pub book: BookKind,
    pub ingredient_count: usize,
    pub has_ingredient_previews: bool,
}
/// The adapter retains the exact native selection privately; consumers see only
/// stable facts. This is not a serialized handle or persistent world identity.
pub trait Selection {
    fn facts(&self) -> &EntryFacts;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    RemoteClient,
    InactiveBook,
    MissingBook,
    MissingEntry,
    UnregisteredDisplay,
    WrongBook,
    DoesNotFit,
    StaleSelection,
}
/// A capability borrowed for one synchronous consumer operation.
///
/// `show` changes ghost presentation only. It does not move/clear inventory,
/// send a placement request, change knowledge, or cancel another caller's
/// placement code. Preview-only consumers must keep their placement vetoes.
pub trait Preview {
    type Selection: Selection;
    fn select(&self, book: BookKind, entry_index: usize) -> Result<Self::Selection, Error>;
    fn show(&mut self, selected: &Self::Selection) -> Result<(), Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayError {
    DuplicateIngredient(usize),
    MissingIngredient(usize),
    DoesNotFit,
}
/// Resolve display alternatives by ingredient index. Empty overrides stay empty;
/// absent overrides retain the supplied native fallback. No eligibility or
/// placement decision is inferred from visual alternatives.
pub fn ingredient_previews<T: Clone>(
    fallback: &[Vec<T>],
    overrides: &[(usize, Vec<T>)],
) -> Result<Vec<Vec<T>>, OverlayError> {
    let mut seen = std::collections::BTreeSet::new();
    for (index, _) in overrides {
        if *index >= fallback.len() {
            return Err(OverlayError::MissingIngredient(*index));
        }
        if !seen.insert(*index) {
            return Err(OverlayError::DuplicateIngredient(*index));
        }
    }
    let mut next = fallback.to_vec();
    for (index, items) in overrides {
        next[*index] = items.clone();
    }
    Ok(next)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overrides_keep_order_explicit_empty_and_native_fallback() {
        let base = vec![vec!["a"], vec!["b"], vec!["c"]];
        assert_eq!(
            ingredient_previews(&base, &[(1, vec!["x", "y"]), (0, vec![])]),
            Ok(vec![vec![], vec!["x", "y"], vec!["c"]])
        );
        assert_eq!(base, vec![vec!["a"], vec!["b"], vec!["c"]]);
    }
    #[test]
    fn invalid_maps_fail_without_changing_inputs() {
        let base = vec![vec!["a"]];
        assert_eq!(
            ingredient_previews(&base, &[(1, vec!["x"])]),
            Err(OverlayError::MissingIngredient(1))
        );
        assert_eq!(
            ingredient_previews(&base, &[(0, vec![]), (0, vec!["x"])]),
            Err(OverlayError::DuplicateIngredient(0))
        );
        assert_eq!(base, vec![vec!["a"]]);
    }
}

//! Explicit, bounded ingredient-presentation changes to an owned book model.
//!
//! Payloads are moved, never decoded, reconstructed or cloned by this service.
//! This is visual model staging, not recipe eligibility, placement or ownership.
use std::collections::BTreeSet;

pub const MAX_ENTRIES: usize = 8_192;
pub const MAX_INGREDIENTS_PER_ENTRY: usize = 1_024;
pub const MAX_OVERRIDES: usize = 65_536;
pub const MAX_ALTERNATIVES: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Crafting,
    /// Native furnace ghost presentation does not consume ingredient overrides.
    Furnace,
}

/// A current positional assertion, not a persistent token or complete definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address {
    pub index: usize,
    pub recipe_id: String,
    pub display_id: i32,
    pub ingredient_count: usize,
    pub kind: Kind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryFacts<'a> {
    pub index: usize,
    pub recipe_id: &'a str,
    pub display_id: i32,
    pub ingredient_count: usize,
    pub kind: Kind,
}
impl EntryFacts<'_> {
    pub fn matches(&self, address: &Address) -> bool {
        self.index == address.index
            && self.recipe_id == address.recipe_id
            && self.display_id == address.display_id
            && self.ingredient_count == address.ingredient_count
            && self.kind == address.kind
    }
}

/// Replace this entry's complete ingredient-override map. Omitted entries stay
/// unchanged. An empty map clears overrides; an empty alternative list is an
/// explicit empty ingredient preview, not a request for native fallback.
/// Ingredient indices follow native requirements(), excluding shaped holes.
pub struct Replacement<T> {
    pub entry: Address,
    pub alternatives: Vec<(usize, Vec<T>)>,
}

impl<T> Replacement<T> {
    /// Start an owned proposal from facts borrowed in the current staging scope.
    /// Only the recipe ID string is copied; stack payloads are never cloned.
    /// An unchanged empty proposal clears this entry's overrides when applied.
    /// This is not a registration, lifetime token or placement permission.
    pub fn for_entry(entry: EntryFacts<'_>) -> Self {
        Self {
            entry: Address {
                index: entry.index,
                recipe_id: entry.recipe_id.to_owned(),
                display_id: entry.display_id,
                ingredient_count: entry.ingredient_count,
                kind: entry.kind,
            },
            alternatives: Vec::new(),
        }
    }

    /// Append one owned ingredient-alternative list without cloning its stacks.
    /// Ordering and explicit empty lists are retained. Duplicate/out-of-range
    /// indices are not silently merged: replace validates the complete batch.
    /// Caller construction/allocation occurs before the staging service's bounds.
    pub fn with_ingredient(mut self, index: usize, alternatives: Vec<T>) -> Self {
        self.alternatives.push((index, alternatives));
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    TooManyEntries,
    TooManyIngredients(usize),
    TooManyOverrides,
    TooManyAlternatives,
    DuplicateEntry(usize),
    MissingEntry(usize),
    AddressMismatch(usize),
    UnsupportedFurnace(usize),
    DuplicateIngredient { entry: usize, ingredient: usize },
    MissingIngredient { entry: usize, ingredient: usize },
}

/// Recoverable validation rejection returns the untouched owned proposals.
/// There is no rollback guarantee for process abort, OOM, panic or payload Drop.
pub struct Rejected<T> {
    pub error: Error,
    pub replacements: Vec<Replacement<T>>,
}

/// A synchronous capability over a detached, exclusively borrowed book model.
/// No Game, registry, retained actor or automatic callback is provided.
pub trait Presentation {
    type Stack;
    fn entry(&self, index: usize) -> Option<EntryFacts<'_>>;
    fn alternatives(&self, index: usize) -> Option<&[(usize, Vec<Self::Stack>)]>;
    /// Validate the whole batch before modifying any entry, then move the exact
    /// owned stacks into presentation only. Never grants placement permission.
    fn replace(
        &mut self,
        replacements: Vec<Replacement<Self::Stack>>,
    ) -> Result<(), Rejected<Self::Stack>>;
}

/// Structural validation only. Lookup is called synchronously in request order,
/// at most once per entry. The native adapter supplies immutable current facts.
/// Limits bound addresses and alternative counts, not caller-owned payload bytes
/// or construction work. No payload is inspected, cloned, encoded or dropped.
pub fn validate<'a, T>(
    replacements: &[Replacement<T>],
    mut current: impl FnMut(usize) -> Option<EntryFacts<'a>>,
) -> Result<(), Error> {
    if replacements.len() > MAX_ENTRIES {
        return Err(Error::TooManyEntries);
    }
    let mut entries = BTreeSet::new();
    let mut total_overrides = 0usize;
    let mut total_alternatives = 0usize;
    for replacement in replacements {
        let index = replacement.entry.index;
        if !entries.insert(index) {
            return Err(Error::DuplicateEntry(index));
        }
        let facts = current(index).ok_or(Error::MissingEntry(index))?;
        if !facts.matches(&replacement.entry) {
            return Err(Error::AddressMismatch(index));
        }
        if facts.kind == Kind::Furnace {
            return Err(Error::UnsupportedFurnace(index));
        }
        if facts.ingredient_count > MAX_INGREDIENTS_PER_ENTRY {
            return Err(Error::TooManyIngredients(index));
        }
        total_overrides = total_overrides
            .checked_add(replacement.alternatives.len())
            .filter(|&n| n <= MAX_OVERRIDES)
            .ok_or(Error::TooManyOverrides)?;
        let mut ingredients = BTreeSet::new();
        for (ingredient, values) in &replacement.alternatives {
            if *ingredient >= facts.ingredient_count {
                return Err(Error::MissingIngredient { entry: index, ingredient: *ingredient });
            }
            if !ingredients.insert(*ingredient) {
                return Err(Error::DuplicateIngredient { entry: index, ingredient: *ingredient });
            }
            total_alternatives = total_alternatives
                .checked_add(values.len())
                .filter(|&n| n <= MAX_ALTERNATIVES)
                .ok_or(Error::TooManyAlternatives)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn facts(index: usize) -> EntryFacts<'static> {
        EntryFacts { index, recipe_id: "example:component_recipe", display_id: -1,
            ingredient_count: 2, kind: Kind::Crafting }
    }
    fn address(index: usize) -> Address {
        let f = facts(index);
        Address { index, recipe_id: f.recipe_id.into(), display_id: f.display_id,
            ingredient_count: f.ingredient_count, kind: f.kind }
    }
    fn request(index: usize, alternatives: Vec<(usize, Vec<u8>)>) -> Replacement<u8> {
        Replacement { entry: address(index), alternatives }
    }
    #[test]
    fn fluent_proposal_copies_current_address_and_moves_nonclone_values() {
        struct NotClone(u8);
        let values = vec![NotClone(7)];
        let pointer = values.as_ptr();
        let edit = Replacement::for_entry(facts(3))
            .with_ingredient(1, values)
            .with_ingredient(0, Vec::new());
        assert_eq!(edit.entry, address(3));
        assert_eq!(edit.alternatives[0].1.as_ptr(), pointer);
        assert_eq!(edit.alternatives[0].1[0].0, 7);
        assert_eq!(edit.alternatives[0].0, 1);
        assert!(edit.alternatives[1].1.is_empty());
        assert_eq!(validate(&[edit], |i| Some(facts(i))), Ok(()));
    }
    #[test]
    fn fluent_proposal_keeps_empty_clear_and_duplicate_rejection() {
        let clear = Replacement::<u8>::for_entry(facts(0));
        let direct_clear = request(0, vec![]);
        assert_eq!(clear.entry, direct_clear.entry);
        assert_eq!(clear.alternatives, direct_clear.alternatives);
        assert_eq!(validate(&[clear], |i| Some(facts(i))),
            validate(&[direct_clear], |i| Some(facts(i))));
        let duplicate = Replacement::<u8>::for_entry(facts(0))
            .with_ingredient(0, vec![1]).with_ingredient(0, Vec::new());
        let direct_duplicate = request(0, vec![(0, vec![1]), (0, Vec::new())]);
        assert_eq!(duplicate.entry, direct_duplicate.entry);
        assert_eq!(duplicate.alternatives, direct_duplicate.alternatives);
        let legacy = validate(&[direct_duplicate], |i| Some(facts(i)));
        assert_eq!(legacy, Err(Error::DuplicateIngredient { entry: 0, ingredient: 0 }));
        assert_eq!(validate(&[duplicate], |i| Some(facts(i))), legacy);
    }
    #[test]
    fn fluent_proposal_is_not_a_freshness_or_kind_bypass() {
        let edit = Replacement::<u8>::for_entry(facts(0)).with_ingredient(0, vec![1]);
        assert_eq!(validate(&[edit], |_| Some(facts(1))), Err(Error::AddressMismatch(0)));
        let mut furnace = facts(0); furnace.kind = Kind::Furnace;
        let edit = Replacement::<u8>::for_entry(furnace);
        assert_eq!(validate(&[edit], |_| Some(furnace)), Err(Error::UnsupportedFurnace(0)));
    }
    #[test]
    fn empty_batch_does_not_lookup_any_entry() {
        assert_eq!(validate::<u8>(&[], |_| panic!("unexpected lookup")), Ok(()));
    }
    #[test]
    fn clear_explicit_empty_order_and_nonclone_payloads_are_supported() {
        struct NotClone;
        let batch = vec![Replacement { entry: address(5), alternatives: vec![(1, vec![NotClone]), (0, vec![])] },
            Replacement { entry: address(2), alternatives: vec![] }];
        let mut seen = Vec::new();
        assert_eq!(validate(&batch, |i| { seen.push(i); Some(facts(i)) }), Ok(()));
        assert_eq!(seen, vec![5, 2]);
        assert_eq!(batch[0].alternatives[0].0, 1);
        assert!(batch[0].alternatives[1].1.is_empty());
        assert!(batch[1].alternatives.is_empty());
    }
    #[test]
    fn duplicate_entry_is_rejected_before_second_lookup() {
        let batch = vec![request(0, vec![]), request(0, vec![])];
        let mut calls = 0;
        assert_eq!(validate(&batch, |i| { calls += 1; Some(facts(i)) }), Err(Error::DuplicateEntry(0)));
        assert_eq!(calls, 1);
    }
    #[test]
    fn duplicate_and_missing_ingredient_are_distinct() {
        assert_eq!(validate(&[request(0, vec![(1, vec![]), (1, vec![1])])], |i| Some(facts(i))),
            Err(Error::DuplicateIngredient { entry: 0, ingredient: 1 }));
        assert_eq!(validate(&[request(0, vec![(2, vec![])])], |i| Some(facts(i))),
            Err(Error::MissingIngredient { entry: 0, ingredient: 2 }));
    }
    #[test]
    fn positional_assertions_are_checked_without_claiming_lifetime() {
        assert_eq!(validate(&[request(0, vec![])], |_| None), Err(Error::MissingEntry(0)));
        for field in 0..5 {
            let mut r = request(0, vec![]);
            match field {
                0 => r.entry.index = 1,
                1 => r.entry.recipe_id.push('x'),
                2 => r.entry.display_id = 4,
                3 => r.entry.ingredient_count = 1,
                _ => r.entry.kind = Kind::Furnace,
            }
            assert_eq!(validate(&[r], |_| Some(facts(0))), Err(Error::AddressMismatch(if field == 0 { 1 } else { 0 })));
        }
    }
    #[test]
    fn furnace_is_rejected_even_for_clear() {
        let mut r = request(0, vec![]); r.entry.kind = Kind::Furnace;
        let mut f = facts(0); f.kind = Kind::Furnace;
        assert_eq!(validate(&[r], |_| Some(f)), Err(Error::UnsupportedFurnace(0)));
    }
    #[test]
    fn entry_and_ingredient_limits_fail_before_payload_work() {
        let batch: Vec<_> = (0..=MAX_ENTRIES).map(|i| request(i, vec![])).collect();
        assert_eq!(validate(&batch, |_| panic!("over-limit lookup")), Err(Error::TooManyEntries));
        let mut r = request(0, vec![]); r.entry.ingredient_count = MAX_INGREDIENTS_PER_ENTRY + 1;
        let mut f = facts(0); f.ingredient_count = r.entry.ingredient_count;
        assert_eq!(validate(&[r], |_| Some(f)), Err(Error::TooManyIngredients(0)));
    }
    #[test]
    fn alternative_budget_is_global_and_exact() {
        let batch = vec![request(0, vec![(0, vec![0; MAX_ALTERNATIVES])]), request(1, vec![(0, vec![1])])];
        assert_eq!(validate(&batch[..1], |i| Some(facts(i))), Ok(()));
        assert_eq!(validate(&batch, |i| Some(facts(i))), Err(Error::TooManyAlternatives));
    }
    #[test]
    fn override_budget_is_global_even_for_empty_lists() {
        let batch: Vec<_> = (0..65).map(|index| {
            let mut entry = address(index); entry.ingredient_count = MAX_INGREDIENTS_PER_ENTRY;
            Replacement::<u8> { entry, alternatives: (0..MAX_INGREDIENTS_PER_ENTRY).map(|i| (i, vec![])).collect() }
        }).collect();
        let lookup = |i| { let mut f = facts(i); f.ingredient_count = MAX_INGREDIENTS_PER_ENTRY; Some(f) };
        assert_eq!(validate(&batch[..64], lookup), Ok(()));
        assert_eq!(validate(&batch, lookup), Err(Error::TooManyOverrides));
    }
}

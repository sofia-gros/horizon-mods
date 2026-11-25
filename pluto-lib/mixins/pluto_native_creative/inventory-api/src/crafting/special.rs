//! Special recipes: providers answer a whole crafting grid when no native
//! recipe matched. A provider inspects the cropped input cells and either
//! passes or supplies the complete result stack together with the recipe
//! identity the native lookup reports for the match. Providers are
//! consulted only after the native lookup found nothing, so a native match
//! always stands. No inventory write is provided; the adapter builds the
//! stack and native code performs consumption.
use crate::advancements::predicates::ComponentValue;
use std::{collections::BTreeMap, sync::RwLock};

use super::matching::InputFacts;

/// The complete result stack a special recipe produces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackSpec {
    pub item: String,
    pub count: u32,
    pub damage: u32,
    /// The result's `minecraft:custom_data` compound, when the recipe's
    /// result carries one.
    pub custom_data: Option<ComponentValue>,
}
/// A match: the recipe identity to report for it and the result stack.
/// The identity is the answering registration's own id in direct use;
/// a forwarder registration returns the underlying provider's id.
pub type Callback = for<'a> fn(&InputFacts<'a>) -> Option<(&'static str, StackSpec)>;
#[derive(Clone, Copy)]
pub struct Registration {
    /// Ordering and duplicate key for this provider. A direct provider's
    /// callback reports this same id as the recipe identity of a match,
    /// in `<namespace>:<path>` form (the id the recipe file would
    /// declare); a forwarder reports the underlying provider's id.
    pub id: &'static str,
    pub callback: Callback,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    EmptyId,
    DuplicateId,
}
struct Registry(RwLock<BTreeMap<&'static str, Callback>>);
impl Registry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, entry: Registration) -> Result<(), RegistrationError> {
        if entry.id.is_empty() {
            return Err(RegistrationError::EmptyId);
        }
        let mut callbacks = self.0.write().unwrap_or_else(|e| e.into_inner());
        if callbacks.contains_key(entry.id) {
            return Err(RegistrationError::DuplicateId);
        }
        callbacks.insert(entry.id, entry.callback);
        Ok(())
    }
    fn has_providers(&self) -> bool {
        !self.0.read().unwrap_or_else(|e| e.into_inner()).is_empty()
    }
    fn evaluate(&self, input: &InputFacts<'_>) -> Option<(&'static str, StackSpec)> {
        let callbacks: Vec<_> = self
            .0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(&id, &callback)| (id, callback))
            .collect();
        // Callbacks run unlocked in lexical registration order; the first
        // provider that answers the grid wins, deterministically.
        callbacks.into_iter().find_map(|(_, callback)| callback(input))
    }
}
static PROVIDERS: Registry = Registry::new();
/// Register once during startup. The callback must be read-only and
/// deterministic for its arguments: the native lookup re-evaluates grids
/// on every inventory change. It must not infer a persistent identity,
/// capture an actor or perform inventory mutations. There is no
/// unregister/reset path.
pub fn register(entry: Registration) -> Result<(), RegistrationError> {
    PROVIDERS.register(entry)
}
pub fn has_providers() -> bool {
    PROVIDERS.has_providers()
}
/// The first answering provider's identity and result stack, if any.
pub fn evaluate(input: &InputFacts<'_>) -> Option<(&'static str, StackSpec)> {
    PROVIDERS.evaluate(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advancements::predicates::ItemSnapshot;

    fn grid<'a>(cells: &'a [Option<ItemSnapshot<'a>>]) -> InputFacts<'a> {
        InputFacts { width: cells.len(), height: 1, left: 0, top: 0, cells }
    }
    fn stack(id: &str) -> Option<ItemSnapshot<'_>> {
        Some(ItemSnapshot {
            id,
            count: 1,
            damage: 0,
            enchantments: &[],
            potion: None,
            custom_name: false,
            components: None,
        })
    }
    fn spec(item: &str) -> StackSpec {
        StackSpec { item: item.to_owned(), count: 1, damage: 0, custom_data: None }
    }
    fn answer_apple(input: &InputFacts<'_>) -> Option<(&'static str, StackSpec)> {
        input
            .cells
            .iter()
            .flatten()
            .any(|cell| cell.id == "example:apple")
            .then(|| ("a-apple", spec("example:pie")))
    }
    fn answer_any(_: &InputFacts<'_>) -> Option<(&'static str, StackSpec)> {
        Some(("z-any", spec("example:any")))
    }
    fn pass(_: &InputFacts<'_>) -> Option<(&'static str, StackSpec)> {
        None
    }

    #[test]
    fn empty_registry_answers_nothing() {
        let r = Registry::new();
        assert!(!r.has_providers());
        assert_eq!(r.evaluate(&grid(&[])), None);
    }

    #[test]
    fn first_answer_in_lexical_order_wins_regardless_of_registration_order() {
        for order in [["z-any", "a-apple"], ["a-apple", "z-any"]] {
            let r = Registry::new();
            for id in order {
                let callback: Callback = if id == "a-apple" { answer_apple } else { answer_any };
                r.register(Registration { id, callback }).unwrap();
            }
            let cells = [stack("example:apple")];
            assert_eq!(
                r.evaluate(&grid(&cells)),
                Some(("a-apple", spec("example:pie"))),
                "the lexical-first provider that answers wins"
            );
            let cells = [stack("example:stone")];
            assert_eq!(r.evaluate(&grid(&cells)), Some(("z-any", spec("example:any"))));
        }
    }

    #[test]
    fn passing_providers_leave_the_grid_unanswered() {
        let r = Registry::new();
        r.register(Registration { id: "pass", callback: pass }).unwrap();
        assert!(r.has_providers());
        assert_eq!(r.evaluate(&grid(&[stack("example:apple")])), None);
    }

    fn answer_one(_: &InputFacts<'_>) -> Option<(&'static str, StackSpec)> {
        Some(("one", spec("example:any")))
    }

    #[test]
    fn duplicates_and_empty_ids_never_replace() {
        let r = Registry::new();
        assert_eq!(
            r.register(Registration { id: "", callback: pass }),
            Err(RegistrationError::EmptyId)
        );
        r.register(Registration { id: "one", callback: answer_one }).unwrap();
        assert_eq!(
            r.register(Registration { id: "one", callback: pass }),
            Err(RegistrationError::DuplicateId)
        );
        assert_eq!(
            r.evaluate(&grid(&[])),
            Some(("one", spec("example:any"))),
            "the original registration still answers"
        );
    }

    #[test]
    fn evaluation_uses_an_unlocked_snapshot() {
        static R: Registry = Registry::new();
        fn answer_later(_: &InputFacts<'_>) -> Option<(&'static str, StackSpec)> {
            Some(("later", spec("example:any")))
        }
        fn callback(_: &InputFacts<'_>) -> Option<(&'static str, StackSpec)> {
            let _ = R.register(Registration { id: "later", callback: answer_later });
            None
        }
        R.register(Registration { id: "first", callback }).unwrap();
        assert_eq!(R.evaluate(&grid(&[])), None, "a registration made mid-evaluation is not in the snapshot");
        assert_eq!(R.evaluate(&grid(&[])), Some(("later", spec("example:any"))));
    }
}

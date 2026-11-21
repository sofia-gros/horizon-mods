//! Recipe extension boundary: custom crafting ingredient kinds and custom
//! crafting assembly for the game's recipe engine, in host-independent form.
//!
//! The recipe files themselves — their folders, the vanilla recipe types,
//! pack merging and the recipe book — belong to the game. What a mod adds is
//! policy the engine cannot know: what a mod-defined ingredient kind (a JSON
//! object in a recipe's `key`/`ingredients` whose `type` names a namespace
//! outside `minecraft:`) matches, and what a crafting grid that a mod's own
//! rules accept produces beyond the recipe's plain result template.
//!
//! Ingredient kinds are consulted twice, as the engine consults them: once
//! at load, where the handler names the item ids the kind can match (item
//! ids only; a kind that claims nothing leaves the list empty and the recipe
//! fails, as an unknown kind does), and again at craft time, where the
//! handler accepts or refuses one concrete stack (its custom data is the
//! usual reason). Assembly providers are consulted when a recipe is about
//! to be assembled from a grid: a provider may pass (the native result
//! stands), deny (the recipe does not match this grid after all), or
//! assemble (the result is the recipe's template stack with its
//! `minecraft:custom_data` component replaced by the supplied compound).
//! Providers never see or mutate the inventory itself.
//!
//! Register once during startup, before recipes load or any craft is
//! evaluated. Callbacks must be read-only and deterministic for their
//! arguments. No content keys or eligibility rules live here.

use std::collections::BTreeMap;
use std::sync::RwLock;

use serde_json::Value;

use super::base::{Nbt, NbtCompound};

/// Borrowed facts about one item stack, valid only for this synchronous
/// evaluation. Missing custom data means unavailable, not an empty stack;
/// consumers requiring it fail closed.
#[derive(Clone, Copy, Debug)]
pub struct StackView<'a> {
    pub id: &'a str,
    pub count: u32,
    pub damage: u32,
    /// The stack's `minecraft:custom_data` compound, when the adapter could
    /// supply it (for a tool or part, its saved tool data).
    pub custom_data: Option<&'a Nbt>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    EmptyId,
    DuplicateId,
}

/* Ingredient kinds */

/// One mod-defined ingredient kind: the two answers the engine needs.
/// `items` runs at recipe load and names every item id the kind can match;
/// `test` runs at craft time against one concrete stack whose item the
/// load-time list already named.
#[derive(Clone, Copy)]
pub struct IngredientKind {
    pub kind: &'static str,
    pub items: fn(&Value) -> Vec<String>,
    pub test: fn(&Value, &StackView<'_>) -> bool,
}

struct IngredientKinds(RwLock<BTreeMap<&'static str, IngredientKind>>);
impl IngredientKinds {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, entry: IngredientKind) -> Result<(), RegistrationError> {
        if entry.kind.is_empty() {
            return Err(RegistrationError::EmptyId);
        }
        let mut kinds = self.0.write().unwrap_or_else(|e| e.into_inner());
        if kinds.contains_key(entry.kind) {
            return Err(RegistrationError::DuplicateId);
        }
        kinds.insert(entry.kind, entry);
        Ok(())
    }
    fn get(&self, kind: &str) -> Option<IngredientKind> {
        self.0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(kind)
            .copied()
    }
    fn has_kinds(&self) -> bool {
        !self.0.read().unwrap_or_else(|e| e.into_inner()).is_empty()
    }
}
static INGREDIENT_KINDS: IngredientKinds = IngredientKinds::new();

/// Register one ingredient kind once during startup. There is no
/// unregister/reset path.
pub fn register_ingredient_kind(entry: IngredientKind) -> Result<(), RegistrationError> {
    INGREDIENT_KINDS.register(entry)
}
pub fn has_ingredient_kinds() -> bool {
    INGREDIENT_KINDS.has_kinds()
}
/// The load-time item list of a claimed kind; None when no handler claims
/// the kind (the engine then leaves the list empty and the recipe fails).
pub fn ingredient_items(kind: &str, data: &Value) -> Option<Vec<String>> {
    INGREDIENT_KINDS.get(kind).map(|entry| (entry.items)(data))
}
/// The craft-time answer of a claimed kind; None when no handler claims the
/// kind (the stack keeps the acceptance the load-time list gave it).
pub fn ingredient_test(kind: &str, data: &Value, stack: &StackView<'_>) -> Option<bool> {
    INGREDIENT_KINDS
        .get(kind)
        .map(|entry| (entry.test)(data, stack))
}

/* Crafting assembly */

/// The form of the recipe being assembled, as the engine parsed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeForm {
    Shaped { width: usize, height: usize },
    Shapeless,
    /// A special recipe: no fixed ingredient list or result template.
    Special,
}

/// Cheap borrowed facts about one recipe, for assembly interest checks.
/// The result fields name the fixed result template; special recipes have
/// none.
#[derive(Clone, Copy, Debug)]
pub struct RecipeFacts<'a> {
    pub form: RecipeForm,
    pub result_item: Option<&'a str>,
    pub result_count: u32,
}

/// The crafting grid as the engine cropped it to the recipe's bounds:
/// row-major cells including empty positions, with the crop's offset in
/// the full grid.
#[derive(Clone, Copy, Debug)]
pub struct GridFacts<'a> {
    pub width: usize,
    pub height: usize,
    pub left: usize,
    pub top: usize,
    pub cells: &'a [Option<StackView<'a>>],
}

pub struct AssemblyRequest<'a> {
    pub recipe: &'a RecipeFacts<'a>,
    pub grid: GridFacts<'a>,
    /// The result template's native created stack, not a proposed output.
    pub result: Option<StackView<'a>>,
}

/// What one assembly provider decides for one grid.
#[derive(Clone, Debug, PartialEq)]
pub enum AssemblyDecision {
    /// Not this provider's recipe, or nothing to change: the native
    /// result stands (unless another provider assembles or denies).
    Pass,
    /// The recipe does not match this grid after all.
    Deny,
    /// The result is the recipe's template stack with its
    /// `minecraft:custom_data` component replaced by this compound.
    Assemble { custom_data: NbtCompound },
}

pub type AssemblyInterest = for<'a> fn(&RecipeFacts<'a>) -> bool;
pub type AssemblyCallback = for<'a> fn(&AssemblyRequest<'a>) -> AssemblyDecision;
#[derive(Clone, Copy)]
pub struct AssemblyRegistration {
    pub id: &'static str,
    pub interest: AssemblyInterest,
    pub callback: AssemblyCallback,
}
#[derive(Clone, Debug, PartialEq)]
pub enum AssemblyOutcome {
    /// No provider assembled or denied: the native result stands.
    Native,
    Denied { first: &'static str },
    Assembled { owner: &'static str, custom_data: NbtCompound },
}
struct AssemblyRegistry(RwLock<BTreeMap<&'static str, (AssemblyInterest, AssemblyCallback)>>);
impl AssemblyRegistry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, entry: AssemblyRegistration) -> Result<(), RegistrationError> {
        if entry.id.is_empty() {
            return Err(RegistrationError::EmptyId);
        }
        let mut callbacks = self.0.write().unwrap_or_else(|e| e.into_inner());
        if callbacks.contains_key(entry.id) {
            return Err(RegistrationError::DuplicateId);
        }
        callbacks.insert(entry.id, (entry.interest, entry.callback));
        Ok(())
    }
    fn has_providers(&self) -> bool {
        !self.0.read().unwrap_or_else(|e| e.into_inner()).is_empty()
    }
    fn prepare(&self, recipe: &RecipeFacts<'_>) -> Option<AssemblyEvaluation> {
        let callbacks: Vec<_> = self
            .0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(&id, &(interest, callback))| (id, interest, callback))
            .collect();
        let selected: Vec<_> = callbacks
            .into_iter()
            .filter_map(|(id, interest, callback)| interest(recipe).then_some((id, callback)))
            .collect();
        (!selected.is_empty()).then_some(AssemblyEvaluation(selected))
    }
}
/// Ephemeral callback snapshot only, not a recipe/world identity or
/// authority token. The adapter evaluates it synchronously with the same
/// recipe facts.
pub struct AssemblyEvaluation(Vec<(&'static str, AssemblyCallback)>);
impl AssemblyEvaluation {
    /// Any denial wins. Otherwise the first assembling provider in lexical
    /// registration order supplies the result's custom data. Callbacks run
    /// unlocked, including after a prior denial or assembly, so the
    /// reported ids are deterministic.
    pub fn evaluate(&self, request: &AssemblyRequest<'_>) -> AssemblyOutcome {
        let mut denied = None;
        let mut assembled = None;
        for &(id, callback) in &self.0 {
            match callback(request) {
                AssemblyDecision::Pass => {}
                AssemblyDecision::Deny => {
                    if denied.is_none() {
                        denied = Some(id);
                    }
                }
                AssemblyDecision::Assemble { custom_data } => {
                    if assembled.is_none() {
                        assembled = Some((id, custom_data));
                    }
                }
            }
        }
        if let Some(first) = denied {
            return AssemblyOutcome::Denied { first };
        }
        assembled.map_or(AssemblyOutcome::Native, |(owner, custom_data)| {
            AssemblyOutcome::Assembled { owner, custom_data }
        })
    }
}
static ASSEMBLERS: AssemblyRegistry = AssemblyRegistry::new();
/// Register once during startup. There is no unregister/reset path.
pub fn register_assembly(entry: AssemblyRegistration) -> Result<(), RegistrationError> {
    ASSEMBLERS.register(entry)
}
pub fn has_assembly_providers() -> bool {
    ASSEMBLERS.has_providers()
}
/// Interest runs on cheap borrowed recipe facts. None avoids all grid and
/// result capture in the adapter, even with unrelated registrations.
pub fn prepare_assembly(recipe: &RecipeFacts<'_>) -> Option<AssemblyEvaluation> {
    ASSEMBLERS.prepare(recipe)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> RecipeFacts<'static> {
        RecipeFacts { form: RecipeForm::Special, result_item: None, result_count: 1 }
    }
    fn request<'a>(recipe: &'a RecipeFacts<'a>) -> AssemblyRequest<'a> {
        AssemblyRequest {
            recipe,
            grid: GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] },
            result: None,
        }
    }
    fn inspect(_: &RecipeFacts<'_>) -> bool {
        true
    }
    fn ignore(_: &RecipeFacts<'_>) -> bool {
        false
    }
    fn pass(_: &AssemblyRequest<'_>) -> AssemblyDecision {
        AssemblyDecision::Pass
    }
    fn deny(_: &AssemblyRequest<'_>) -> AssemblyDecision {
        AssemblyDecision::Deny
    }
    fn assemble(_: &AssemblyRequest<'_>) -> AssemblyDecision {
        let mut data = NbtCompound::new();
        data.insert("made".into(), Nbt::Byte(1));
        AssemblyDecision::Assemble { custom_data: data }
    }

    #[test]
    fn empty_and_uninterested_registries_need_no_evaluation() {
        let r = AssemblyRegistry::new();
        assert!(!r.has_providers());
        assert!(r.prepare(&facts()).is_none());
        r.register(AssemblyRegistration { id: "unrelated", interest: ignore, callback: assemble })
            .unwrap();
        assert!(r.prepare(&facts()).is_none());
    }

    #[test]
    fn denial_wins_and_the_first_assembly_is_deterministic() {
        for order in [["z", "a"], ["a", "z"]] {
            let r = AssemblyRegistry::new();
            r.register(AssemblyRegistration { id: "0", interest: inspect, callback: pass }).unwrap();
            let facts = facts();
            assert_eq!(r.prepare(&facts).unwrap().evaluate(&request(&facts)), AssemblyOutcome::Native);
            for id in order {
                r.register(AssemblyRegistration { id, interest: inspect, callback: assemble }).unwrap();
            }
            let AssemblyOutcome::Assembled { owner, custom_data } =
                r.prepare(&facts).unwrap().evaluate(&request(&facts))
            else {
                panic!("expected an assembly");
            };
            assert_eq!(owner, "a", "lexical order, not registration order");
            assert_eq!(custom_data.get("made"), Some(&Nbt::Byte(1)));
            r.register(AssemblyRegistration { id: "m", interest: inspect, callback: deny }).unwrap();
            assert_eq!(
                r.prepare(&facts).unwrap().evaluate(&request(&facts)),
                AssemblyOutcome::Denied { first: "m" }
            );
        }
    }

    #[test]
    fn duplicate_and_empty_assembly_ids_never_replace() {
        let r = AssemblyRegistry::new();
        assert_eq!(
            r.register(AssemblyRegistration { id: "", interest: inspect, callback: pass }),
            Err(RegistrationError::EmptyId)
        );
        r.register(AssemblyRegistration { id: "one", interest: inspect, callback: deny }).unwrap();
        assert_eq!(
            r.register(AssemblyRegistration { id: "one", interest: ignore, callback: pass }),
            Err(RegistrationError::DuplicateId)
        );
    }

    #[test]
    fn ingredient_kinds_claim_by_exact_name() {
        let kinds = IngredientKinds::new();
        assert!(!kinds.has_kinds());
        assert!(kinds.get("test:kind").is_none());
        kinds
            .register(IngredientKind {
                kind: "test:kind",
                items: |_| vec!["test:item".to_owned()],
                test: |_, stack| stack.id == "test:item",
            })
            .unwrap();
        assert!(kinds.has_kinds());
        let entry = kinds.get("test:kind").unwrap();
        let data = Value::Object(Default::default());
        assert_eq!((entry.items)(&data), ["test:item"]);
        let stack = StackView { id: "test:item", count: 1, damage: 0, custom_data: None };
        assert!((entry.test)(&data, &stack));
        assert_eq!(
            kinds.register(IngredientKind { kind: "test:kind", items: |_| vec![], test: |_, _| true }),
            Err(RegistrationError::DuplicateId)
        );
        assert_eq!(
            kinds.register(IngredientKind { kind: "", items: |_| vec![], test: |_, _| true }),
            Err(RegistrationError::EmptyId)
        );
    }
}

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
//! Admission policies are the deny-only sibling of assembly: consulted
//! before a recipe is matched or assembled, a policy may pass (the native
//! result stands) or deny (the recipe does not match this grid after all);
//! a policy never assembles. The recipe facts name the recipe's loaded id,
//! so a policy can admit by identity — the shape a config-gated recipe
//! needs, since the game's data conditions cannot name a mod's config
//! values.
//!
//! Special recipes are the third shape: a recipe whose result depends on
//! the grid's concrete stacks (their materials, damage or data) has no
//! fixed template the engine could assemble, and the engine's own special
//! recipe list is closed, so a mod registers the recipe itself. A special
//! provider is consulted only when the native lookup found no match for
//! the grid; it inspects the cropped grid and either passes or supplies
//! the complete result stack. The registration id is the recipe identity
//! the lookup reports, in the `<namespace>:<path>` form the recipe file
//! would declare. Consumption stays native: taking the result consumes
//! one item from every occupied grid cell, as for any crafting recipe.
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
    /// The recipe's id as loaded (`<namespace>:<path>` for a content mod's
    /// recipe, the bare name for one of vanilla's), when the adapter could
    /// name it; None when it could not.
    pub id: Option<&'a str>,
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

/* Crafting admission */

/// What an admission callback answers: the recipe's facts, the cropped
/// grid, and the result template's native created stack (not a proposed
/// output).
pub struct AdmissionRequest<'a> {
    pub recipe: &'a RecipeFacts<'a>,
    pub grid: GridFacts<'a>,
    pub result: Option<StackView<'a>>,
}

/// What one admission policy decides for one grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionDecision {
    /// Not this policy's recipe, or nothing to refuse: the native result
    /// stands (unless another policy denies).
    Pass,
    /// The recipe does not match this grid after all.
    Deny,
}

pub type AdmissionInterest = for<'a> fn(&RecipeFacts<'a>) -> bool;
pub type AdmissionCallback = for<'a> fn(&AdmissionRequest<'a>) -> AdmissionDecision;
#[derive(Clone, Copy)]
pub struct AdmissionRegistration {
    pub id: &'static str,
    pub interest: AdmissionInterest,
    pub callback: AdmissionCallback,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdmissionOutcome {
    /// No policy denied: the native result stands.
    Native,
    Denied { first: &'static str },
}
struct AdmissionRegistry(RwLock<BTreeMap<&'static str, (AdmissionInterest, AdmissionCallback)>>);
impl AdmissionRegistry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, entry: AdmissionRegistration) -> Result<(), RegistrationError> {
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
    fn prepare(&self, recipe: &RecipeFacts<'_>) -> Option<AdmissionEvaluation> {
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
        (!selected.is_empty()).then_some(AdmissionEvaluation(selected))
    }
}
/// Ephemeral callback snapshot only, not a recipe/world identity or
/// authority token. The adapter evaluates it synchronously with the same
/// recipe facts.
pub struct AdmissionEvaluation(Vec<(&'static str, AdmissionCallback)>);
impl AdmissionEvaluation {
    /// Any denial wins. Pass cannot grant matching or provide an output.
    /// Callbacks run unlocked in lexical registration order, including
    /// after a prior denial; the first denying ID is reported
    /// deterministically.
    pub fn evaluate(&self, request: &AdmissionRequest<'_>) -> AdmissionOutcome {
        let mut first = None;
        for &(id, callback) in &self.0 {
            if callback(request) == AdmissionDecision::Deny && first.is_none() {
                first = Some(id);
            }
        }
        first.map_or(AdmissionOutcome::Native, |first| AdmissionOutcome::Denied { first })
    }
}
static ADMISSIONS: AdmissionRegistry = AdmissionRegistry::new();
/// Register one admission policy once during startup. Both functions must
/// be read-only and deterministic for their arguments. There is no
/// unregister/reset path.
pub fn register_admission(entry: AdmissionRegistration) -> Result<(), RegistrationError> {
    ADMISSIONS.register(entry)
}
pub fn has_admission_providers() -> bool {
    ADMISSIONS.has_providers()
}
/// Interest runs on cheap borrowed recipe facts. None avoids all grid and
/// result capture in the adapter, even with unrelated registrations.
pub fn prepare_admission(recipe: &RecipeFacts<'_>) -> Option<AdmissionEvaluation> {
    ADMISSIONS.prepare(recipe)
}

/* Special recipes */

/// The complete result stack a special recipe produces for one grid.
#[derive(Clone, Debug, PartialEq)]
pub struct SpecialResult {
    pub item: String,
    pub count: u32,
    pub damage: u32,
    /// The result's `minecraft:custom_data` compound, when the result
    /// carries one (a repaired tool's saved tool data, for instance).
    pub custom_data: Option<NbtCompound>,
}

pub type SpecialCallback = for<'a> fn(&GridFacts<'a>) -> Option<SpecialResult>;
#[derive(Clone, Copy)]
pub struct SpecialRecipeRegistration {
    /// The recipe identity reported for a match, in `<namespace>:<path>`
    /// form (the id the recipe file would declare).
    pub id: &'static str,
    pub callback: SpecialCallback,
}
struct SpecialRegistry(RwLock<BTreeMap<&'static str, SpecialCallback>>);
impl SpecialRegistry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, entry: SpecialRecipeRegistration) -> Result<(), RegistrationError> {
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
    fn evaluate(&self, grid: &GridFacts<'_>) -> Option<(&'static str, SpecialResult)> {
        let callbacks: Vec<_> = self
            .0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(&id, &callback)| (id, callback))
            .collect();
        // Callbacks run unlocked in lexical registration order; the first
        // provider that answers the grid wins, deterministically.
        callbacks
            .into_iter()
            .find_map(|(id, callback)| callback(grid).map(|result| (id, result)))
    }
}
static SPECIAL_RECIPES: SpecialRegistry = SpecialRegistry::new();
/// Register one special recipe once during startup. The callback must be
/// read-only and deterministic for its arguments: the lookup re-evaluates
/// grids on every inventory change. There is no unregister/reset path.
pub fn register_special_recipe(
    entry: SpecialRecipeRegistration,
) -> Result<(), RegistrationError> {
    SPECIAL_RECIPES.register(entry)
}
pub fn has_special_recipes() -> bool {
    SPECIAL_RECIPES.has_providers()
}
/// The first answering provider's recipe identity and result, if any.
pub fn evaluate_special_recipe(
    grid: &GridFacts<'_>,
) -> Option<(&'static str, SpecialResult)> {
    SPECIAL_RECIPES.evaluate(grid)
}
pub fn has_assembly_providers() -> bool {
    ASSEMBLERS.has_providers()
}

/* Cooking lookups */

/// One cooking recipe of the game's own book (the furnace family), as a lookup answers it.
#[derive(Clone, Debug, PartialEq)]
pub struct CookingRecipe {
    /// The result's item id.
    pub result: String,
    /// The result's count for one input item.
    pub result_count: u32,
    /// The experience the recipe grants per item cooked.
    pub experience: f32,
    /// The cooking time in ticks.
    pub time: u32,
}

/// The adapter's cooking lookup: the first recipe of a kind (a recipe type id such as
/// `minecraft:smelting`) that takes an item, or None when the book has none.
pub type CookingLookup = fn(&str, &str) -> Option<CookingRecipe>;

static COOKING_LOOKUP: RwLock<Option<CookingLookup>> = RwLock::new(None);

/// Install the adapter's cooking lookup, once during startup. There is no uninstall.
pub fn set_cooking_lookup(lookup: CookingLookup) {
    if let Ok(mut slot) = COOKING_LOOKUP.write() {
        *slot = Some(lookup);
    }
}

/// The first cooking recipe of `kind` that takes `item`, as the game's recipe book answers it
/// (its ingredient naming the item; a stack-sensitive ingredient test is the engine's and is not
/// replayed). None when the book has none, or when no adapter is installed.
pub fn cooking_recipe(kind: &str, item: &str) -> Option<CookingRecipe> {
    let lookup = COOKING_LOOKUP.read().unwrap_or_else(|e| e.into_inner());
    (*lookup)?(kind, item)
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
        RecipeFacts { form: RecipeForm::Special, result_item: None, result_count: 1, id: None }
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

    fn admission_request<'a>(recipe: &'a RecipeFacts<'a>) -> AdmissionRequest<'a> {
        AdmissionRequest {
            recipe,
            grid: GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] },
            result: None,
        }
    }
    fn pass_admission(_: &AdmissionRequest<'_>) -> AdmissionDecision {
        AdmissionDecision::Pass
    }
    fn deny_admission(_: &AdmissionRequest<'_>) -> AdmissionDecision {
        AdmissionDecision::Deny
    }

    #[test]
    fn admission_denial_wins_in_lexical_order() {
        let empty = AdmissionRegistry::new();
        assert!(!empty.has_providers());
        assert!(empty.prepare(&facts()).is_none());
        empty
            .register(AdmissionRegistration { id: "unrelated", interest: ignore, callback: deny_admission })
            .unwrap();
        assert!(empty.prepare(&facts()).is_none());
        for order in [["z", "a"], ["a", "z"]] {
            let r = AdmissionRegistry::new();
            r.register(AdmissionRegistration { id: "0", interest: inspect, callback: pass_admission }).unwrap();
            let facts = facts();
            assert_eq!(r.prepare(&facts).unwrap().evaluate(&admission_request(&facts)), AdmissionOutcome::Native);
            for id in order {
                r.register(AdmissionRegistration { id, interest: inspect, callback: deny_admission }).unwrap();
            }
            assert_eq!(
                r.prepare(&facts).unwrap().evaluate(&admission_request(&facts)),
                AdmissionOutcome::Denied { first: "a" },
                "lexical order, not registration order"
            );
        }
    }

    #[test]
    fn admission_ids_are_unique_and_nonempty() {
        let r = AdmissionRegistry::new();
        assert_eq!(
            r.register(AdmissionRegistration { id: "", interest: inspect, callback: pass_admission }),
            Err(RegistrationError::EmptyId)
        );
        r.register(AdmissionRegistration { id: "one", interest: inspect, callback: deny_admission }).unwrap();
        assert_eq!(
            r.register(AdmissionRegistration { id: "one", interest: ignore, callback: pass_admission }),
            Err(RegistrationError::DuplicateId)
        );
    }

    fn special_grid<'a>(cells: &'a [Option<StackView<'a>>]) -> GridFacts<'a> {
        GridFacts { width: cells.len(), height: 1, left: 0, top: 0, cells }
    }
    fn special_result(item: &str) -> SpecialResult {
        SpecialResult { item: item.to_owned(), count: 1, damage: 0, custom_data: None }
    }
    fn answer_apple(grid: &GridFacts<'_>) -> Option<SpecialResult> {
        grid.cells
            .iter()
            .flatten()
            .any(|cell| cell.id == "test:apple")
            .then(|| special_result("test:pie"))
    }
    fn answer_any(_: &GridFacts<'_>) -> Option<SpecialResult> {
        Some(special_result("test:any"))
    }
    fn pass_special(_: &GridFacts<'_>) -> Option<SpecialResult> {
        None
    }

    #[test]
    fn special_recipes_answer_in_lexical_order_and_pass_through() {
        let stack = StackView { id: "test:apple", count: 1, damage: 0, custom_data: None };
        let cells = [Some(stack)];
        let empty = SpecialRegistry::new();
        assert!(!empty.has_providers());
        assert_eq!(empty.evaluate(&special_grid(&cells)), None);
        for order in [["z:any", "a:apple"], ["a:apple", "z:any"]] {
            let r = SpecialRegistry::new();
            for id in order {
                let callback: SpecialCallback = if id == "a:apple" { answer_apple } else { answer_any };
                r.register(SpecialRecipeRegistration { id, callback }).unwrap();
            }
            assert_eq!(
                r.evaluate(&special_grid(&cells)),
                Some(("a:apple", special_result("test:pie"))),
                "the lexical-first provider that answers wins"
            );
            let stone = [Some(StackView { id: "test:stone", ..stack })];
            assert_eq!(r.evaluate(&special_grid(&stone)), Some(("z:any", special_result("test:any"))));
        }
        let passing = SpecialRegistry::new();
        passing.register(SpecialRecipeRegistration { id: "pass", callback: pass_special }).unwrap();
        assert!(passing.has_providers());
        assert_eq!(passing.evaluate(&special_grid(&cells)), None);
    }

    #[test]
    fn special_recipe_ids_are_unique_and_nonempty() {
        let r = SpecialRegistry::new();
        assert_eq!(
            r.register(SpecialRecipeRegistration { id: "", callback: pass_special }),
            Err(RegistrationError::EmptyId)
        );
        r.register(SpecialRecipeRegistration { id: "one", callback: answer_any }).unwrap();
        assert_eq!(
            r.register(SpecialRecipeRegistration { id: "one", callback: pass_special }),
            Err(RegistrationError::DuplicateId)
        );
        assert_eq!(r.evaluate(&special_grid(&[])), Some(("one", special_result("test:any"))));
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

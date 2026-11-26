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
//! one item from every occupied grid cell, as for any crafting recipe —
//! except the cells a provider's declared remainders leave in place,
//! which the crafting lookup below applies.
//!
//! The crafting lookup is the read side of the same book: given a grid,
//! the adapter answers the recipe the game's own crafting would match,
//! its assembled result and what each cell holds after one craft, so a
//! mod's own crafting surface (a persistent station grid, say) crafts
//! with the game's semantics instead of beside them.
//!
//! Register once during startup, before recipes load or any craft is
//! evaluated. Callbacks must be read-only and deterministic for their
//! arguments. No content keys or eligibility rules live here.

use std::collections::BTreeMap;
use std::sync::RwLock;

use serde_json::Value;

use super::base::{Identifier, Nbt, NbtCompound};
use super::block_entity_api_v1::ItemSlot;

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
    /// The cells of the evaluated (cropped) grid this craft does not
    /// spend, aligned with the grid the callback was given: an entry
    /// names the stack that stays in that cell — the shape of a recipe
    /// whose `getRemainingItems` leaves an unneeded ingredient in place
    /// once the craft's need is covered (a repair that only spends as
    /// many repair items as the damage requires). The field or an entry
    /// being `None` spends the cell, and the engine's own remainder for
    /// it (a container item) stands. [`crafting_match`] applies the
    /// stays over the engine's remainders; the game's own take path
    /// does not consult providers, so a special crafted there still
    /// spends every occupied cell.
    pub remainders: Option<Vec<Option<SpecialResult>>>,
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
    fn ids(&self) -> Vec<&'static str> {
        self.0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .copied()
            .collect()
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
    SPECIAL_RECIPES.register(entry)?;
    if let Ok(slot) = SPECIAL_RECIPE_SINK.read() {
        if let Some(sink) = *slot {
            sink(entry.id);
        }
    }
    Ok(())
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
/// The ids of every registered special recipe, in lexical order. The
/// game glue mirrors each id into the native inventory registry so a
/// provider can be resolved under its own recipe name.
pub fn special_recipe_ids() -> Vec<&'static str> {
    SPECIAL_RECIPES.ids()
}

/// The sink notified with the id of each special recipe as it
/// registers: the game glue's mirror into the native inventory
/// registry. Registrations from before the sink was installed are
/// covered by [`special_recipe_ids`].
pub type SpecialRecipeSink = fn(&'static str);

static SPECIAL_RECIPE_SINK: RwLock<Option<SpecialRecipeSink>> = RwLock::new(None);

/// Install the special-recipe registration sink, once during startup.
/// There is no uninstall.
pub fn set_special_recipe_sink(sink: SpecialRecipeSink) {
    if let Ok(mut slot) = SPECIAL_RECIPE_SINK.write() {
        *slot = Some(sink);
    }
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

/* Crafting lookups */

/// One match of the game's crafting book for a grid, as [`crafting_match`] answers it.
#[derive(Clone, Debug, PartialEq)]
pub struct CraftingMatch {
    /// The matched recipe's id, qualified (`minecraft:oak_planks` for a
    /// bare table name, the whole id for anyone else's recipe).
    pub recipe: String,
    /// The assembled result stack.
    pub result: ItemSlot,
    /// What each grid cell holds after one craft, aligned with the
    /// queried grid (`size * size` entries, row-major): a spent cell's
    /// crafting remainder (a milk bucket's empty bucket), or None where
    /// the cell empties. Taking the result spends one item from every
    /// occupied cell and leaves these stacks in their cells; a remainder
    /// that cannot stay in a shrunken cell is the taker's to hand to the
    /// player, as in the game's own take logic.
    pub remainders: Vec<Option<ItemSlot>>,
}

/// The adapter's crafting lookup: the crafting book's match for a grid
/// of `size` rows and columns (the cells row-major, `size * size` of
/// them), or None when the book matches nothing.
pub type CraftingLookup = fn(&[Option<ItemSlot>], usize) -> Option<CraftingMatch>;

static CRAFTING_LOOKUP: RwLock<Option<CraftingLookup>> = RwLock::new(None);

/// Install the adapter's crafting lookup, once during startup. There is no uninstall.
pub fn set_crafting_lookup(lookup: CraftingLookup) {
    if let Ok(mut slot) = CRAFTING_LOOKUP.write() {
        *slot = Some(lookup);
    }
}

/// The crafting book's match for `grid`, as the game's own crafting
/// answers it: the first recipe in the book's order whose match the
/// grid satisfies — admission policies, assembly providers and special
/// markers all apply, as they do at the game's own table — assembled
/// for this grid. A matched special recipe whose provider declares
/// remainders leaves those cells' stays in place of the engine's
/// remainders. None when the book matches nothing, or when no adapter
/// is installed.
pub fn crafting_match(grid: &[Option<ItemSlot>], size: usize) -> Option<CraftingMatch> {
    let lookup = CRAFTING_LOOKUP.read().unwrap_or_else(|e| e.into_inner());
    let mut matched = (*lookup)?(grid, size)?;
    if SPECIAL_RECIPES.has_providers() {
        if let Some(stays) = special_stays(&matched.recipe, grid, size) {
            for (cell, stay) in stays {
                if let Some(slot) = matched.remainders.get_mut(cell) {
                    *slot = stay;
                }
            }
        }
    }
    Some(matched)
}

/// The stays a matched special recipe's provider declares for this
/// grid, as (full-grid cell, stack) pairs: the provider is re-asked
/// over the cropped grid, as the engine's own input crops it, and only
/// its answer under the matched recipe's own id counts.
fn special_stays(
    recipe: &str,
    grid: &[Option<ItemSlot>],
    size: usize,
) -> Option<Vec<(usize, Option<ItemSlot>)>> {
    // The occupied bounding box, cropped as the engine's own input crops.
    let (mut left, mut top, mut right, mut bottom) = (size, size, 0, 0);
    let mut any = false;
    for row in 0..size {
        for col in 0..size {
            if grid.get(row * size + col).is_some_and(|cell| cell.is_some()) {
                any = true;
                left = left.min(col);
                top = top.min(row);
                right = right.max(col);
                bottom = bottom.max(row);
            }
        }
    }
    if !any {
        return None;
    }
    let width = right + 1 - left;
    let height = bottom + 1 - top;
    let cropped: Vec<Option<ItemSlot>> = (0..width * height)
        .map(|index| {
            grid.get((top + index / width) * size + (left + index % width))
                .cloned()
                .flatten()
        })
        .collect();
    let ids: Vec<Option<String>> = cropped
        .iter()
        .map(|cell| cell.as_ref().map(|slot| slot.item.to_string()))
        .collect();
    let datas: Vec<Option<Nbt>> = cropped
        .iter()
        .map(|cell| {
            cell.as_ref().and_then(|slot| {
                (!slot.custom_data.is_empty()).then(|| Nbt::Compound(slot.custom_data.clone()))
            })
        })
        .collect();
    let cells: Vec<Option<StackView<'_>>> = cropped
        .iter()
        .zip(ids.iter())
        .zip(datas.iter())
        .map(|((slot, id), data)| {
            slot.as_ref().map(|slot| StackView {
                id: id.as_deref().unwrap_or_default(),
                count: slot.count,
                damage: slot.damage,
                custom_data: data.as_ref(),
            })
        })
        .collect();
    let facts = GridFacts { width, height, left, top, cells: &cells };
    let (id, result) = SPECIAL_RECIPES.evaluate(&facts)?;
    if id != recipe {
        return None;
    }
    let declared = result.remainders?;
    let mut stays = Vec::new();
    for (index, stay) in declared.into_iter().enumerate() {
        let Some(stay) = stay else { continue };
        let cell = (top + index / width) * size + (left + index % width);
        stays.push((cell, Some(stay_slot(&stay)?)));
    }
    Some(stays)
}

/// A declared stay as a slot: the result shape's own conversion, as the
/// engine's special seam converts a provider's result.
fn stay_slot(stay: &SpecialResult) -> Option<ItemSlot> {
    let mut slot = ItemSlot::new(Identifier::parse(&stay.item).ok()?, stay.count);
    slot.damage = stay.damage;
    if let Some(data) = &stay.custom_data {
        slot.custom_data = data.clone();
    }
    Some(slot)
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
        SpecialResult { item: item.to_owned(), count: 1, damage: 0, custom_data: None, remainders: None }
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
        assert_eq!(r.ids(), vec!["one"]);
    }

    #[test]
    fn special_recipe_sink_and_ids_track_registrations() {
        use std::sync::Mutex;
        static SEEN: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
        fn record(id: &'static str) {
            SEEN.lock().unwrap().push(id);
        }
        set_special_recipe_sink(record);
        register_special_recipe(SpecialRecipeRegistration { id: "sinktest:one", callback: pass_special })
            .unwrap();
        assert!(special_recipe_ids().contains(&"sinktest:one"));
        assert!(SEEN.lock().unwrap().contains(&"sinktest:one"));
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

    fn slot(item: &str, count: u32) -> ItemSlot {
        ItemSlot::new(Identifier::parse(item).unwrap(), count)
    }

    /// A two-recipe book: four planks in a square craft a table (no
    /// remainders), and the stay special's marker matches a tool beside
    /// a bottle anywhere in the grid.
    fn book_lookup(grid: &[Option<ItemSlot>], size: usize) -> Option<CraftingMatch> {
        let cell = |index: usize| grid.get(index).cloned().flatten();
        let planks = [0, 1, size, size + 1]
            .iter()
            .all(|&index| cell(index).is_some_and(|stack| stack.item.to_string() == "minecraft:oak_planks"));
        if planks {
            return Some(CraftingMatch {
                recipe: "minecraft:crafting_table".to_owned(),
                result: slot("minecraft:crafting_table", 1),
                remainders: vec![None; size * size],
            });
        }
        let tool = grid.iter().flatten().any(|stack| stack.item.to_string() == "test:tool");
        let bottle = grid.iter().flatten().any(|stack| stack.item.to_string() == "test:bottle");
        if tool && bottle {
            return Some(CraftingMatch {
                recipe: "test:stay_special".to_owned(),
                result: slot("test:tool", 1),
                remainders: vec![None; size * size],
            });
        }
        None
    }

    /// The stay special: the bottle beside the tool stays in its cell.
    fn answer_stay(grid: &GridFacts<'_>) -> Option<SpecialResult> {
        let tool = grid.cells.iter().flatten().any(|cell| cell.id == "test:tool");
        let bottle = grid.cells.iter().position(|cell| cell.is_some_and(|cell| cell.id == "test:bottle"));
        let (true, Some(bottle)) = (tool, bottle) else { return None };
        let mut remainders: Vec<Option<SpecialResult>> = vec![None; grid.cells.len()];
        remainders[bottle] = Some(special_result("test:bottle"));
        let mut result = special_result("test:tool");
        result.remainders = Some(remainders);
        Some(result)
    }

    #[test]
    fn crafting_match_passes_the_book_answer_through() {
        set_crafting_lookup(book_lookup);
        let mut grid = vec![None; 9];
        for index in [0, 1, 3, 4] {
            grid[index] = Some(slot("minecraft:oak_planks", 2));
        }
        let matched = crafting_match(&grid, 3).expect("the square of planks matches");
        assert_eq!(matched.recipe, "minecraft:crafting_table");
        assert_eq!(matched.result, slot("minecraft:crafting_table", 1));
        assert_eq!(matched.remainders, vec![None; 9]);
        let mut unmatched = vec![None; 9];
        unmatched[0] = Some(slot("minecraft:oak_planks", 1));
        assert_eq!(crafting_match(&unmatched, 3), None, "a lone plank matches nothing");
    }

    #[test]
    fn crafting_match_applies_a_specials_declared_stays() {
        set_crafting_lookup(book_lookup);
        register_special_recipe(SpecialRecipeRegistration { id: "test:stay_special", callback: answer_stay })
            .expect("first registration of the stay special");
        // The tool at (1, 1) and the bottle at (1, 2): the cropped box
        // starts at column 1, so the stay must land on full-grid cell 5.
        let mut grid = vec![None; 9];
        grid[4] = Some(slot("test:tool", 1));
        grid[5] = Some(slot("test:bottle", 3));
        let matched = crafting_match(&grid, 3).expect("the marker matches");
        assert_eq!(matched.recipe, "test:stay_special");
        assert_eq!(matched.remainders[5], Some(slot("test:bottle", 1)), "the unneeded bottle stays");
        assert_eq!(matched.remainders[4], None, "the tool cell is spent");
    }
}

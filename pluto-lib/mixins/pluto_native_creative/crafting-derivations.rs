// Native full-stack derivation authority, separate from matching admission.
// Callbacks must be deterministic pure functions of the supplied native grid.
// Assembly and remainder requests recompute independently; there is no receipt,
// slot identity, cache, transaction, or same-invocation consumption guarantee.
use super::{Input, Recipe};
use horizon_items::items::ItemStack;
use pluto_inventory_api::crafting::products::Pair;
use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::sync::{LazyLock, RwLock};

pub type Derive = fn(&[Option<ItemStack>]) -> Option<Pair<ItemStack>>;
#[derive(Clone, Copy)]
pub struct PureRecipe {
    pub id: &'static str,
    pub assembly_interest: fn(&Recipe) -> bool,
    pub remainder_interest: fn(&str) -> bool,
    pub derive: Derive,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError { EmptyId, DuplicateId }
#[derive(Default)]
struct Registry { entries: RwLock<BTreeMap<&'static str, PureRecipe>> }
impl Registry {
    fn insert(&self, entry: PureRecipe) -> Result<(), RegistrationError> {
        if entry.id.is_empty() { return Err(RegistrationError::EmptyId); }
        let mut entries = self.entries.write().expect("pure recipe registry poisoned");
        if entries.contains_key(entry.id) { return Err(RegistrationError::DuplicateId); }
        entries.insert(entry.id, entry);
        Ok(())
    }
    fn snapshot(&self) -> Vec<PureRecipe> {
        self.entries.read().expect("pure recipe registry poisoned").values().copied().collect()
    }
}
static RECIPES: LazyLock<Registry> = LazyLock::new(Registry::default);
/// Register selectors and their one pure derivation together. No callback runs
/// under this lock. Register before recipe matching becomes reachable.
pub fn register(entry: PureRecipe) -> Result<(), RegistrationError> { RECIPES.insert(entry) }

enum Selection { Native, One(Derive), Conflict }
fn select(entries: Vec<PureRecipe>, mut interested: impl FnMut(PureRecipe) -> bool) -> Selection {
    let mut selected = None;
    let mut conflict = false;
    // Evaluate all selectors in stable ID order, outside the registry lock.
    for entry in entries {
        if interested(entry) {
            if selected.is_some() { conflict = true; }
            else { selected = Some(entry.derive); }
        }
    }
    if conflict { Selection::Conflict }
    else { selected.map_or(Selection::Native, Selection::One) }
}
fn assembly_from(registry: &Registry, input: &Input<'_>, recipe: &Recipe) -> ControlFlow<Option<Option<ItemStack>>> {
    let selected = select(registry.snapshot(), |entry| (entry.assembly_interest)(recipe));
    let derive = match selected {
        Selection::Native => return ControlFlow::Continue(()),
        Selection::Conflict => return ControlFlow::Break(None),
        Selection::One(derive) => derive,
    };
    // Preserve the old assembly caller's row-major nonempty compression. Clone
    // actual full native values, not ItemFacts or serialized reconstruction.
    let grid: Vec<Option<ItemStack>> = input.stacks().into_iter().map(|stack| Some(stack.clone())).collect();
    let pair = derive(&grid).filter(|pair| pair.remainders.len() == grid.len());
    ControlFlow::Break(pair.map(|pair| Some(pair.output)))
}
/// Called only AFTER every matching veto has admitted this invocation. A claimed
/// derivation bypasses the native matcher exactly as the legacy override did.
/// It is not a Pass/Deny policy and may return an output only through this API.
fn after_matching_from(registry: &Registry, admission: pluto_inventory_api::crafting::matching::Outcome,
    input: &Input<'_>, recipe: &Recipe) -> ControlFlow<Option<Option<ItemStack>>> {
    if matches!(admission, pluto_inventory_api::crafting::matching::Outcome::Denied { .. }) {
        ControlFlow::Break(None)
    } else { assembly_from(registry, input, recipe) }
}
/// The single installed HEAD supplies the actual veto outcome before this call.
/// A denial must not query a derivation selector or invoke its callback.
pub fn after_matching(admission: pluto_inventory_api::crafting::matching::Outcome,
    input: &Input<'_>, recipe: &Recipe) -> ControlFlow<Option<Option<ItemStack>>> {
    after_matching_from(&RECIPES, admission, input, recipe)
}
fn remainders_from(registry: &Registry, recipe: &str, grid: &[Option<ItemStack>], size: usize) -> ControlFlow<Vec<Option<ItemStack>>> {
    let selected = select(registry.snapshot(), |entry| (entry.remainder_interest)(recipe));
    match selected {
        Selection::Native => ControlFlow::Continue(()),
        Selection::Conflict => ControlFlow::Break(vec![None; size * size]),
        Selection::One(derive) => ControlFlow::Break(derive(grid)
            .filter(|pair| pair.remainders.len() == grid.len())
            .map(|pair| pair.remainders)
            .unwrap_or_else(|| vec![None; size * size])),
    }
}
/// Independent pure recomputation over the original grid, not reuse of assembly
/// state. Native consumption/quick-move/actor/network code remains its owner.
pub fn remainders(recipe: &str, grid: &[Option<ItemStack>], size: usize) -> ControlFlow<Vec<Option<ItemStack>>> {
    remainders_from(&RECIPES, recipe, grid, size)
}

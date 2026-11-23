// The game's own cooking recipes (the furnace family: smelting, blasting, smoking and campfire
// cooking), published when the recipe book loads, so a mod can ask what a furnace does to an
// item (autosmelt's drop conversion, a frypan's cooking) without the recipe engine. The table
// keeps the book's order, and a lookup answers the first recipe of the kind whose ingredient
// names the item, as the book's own first-match lookup answers; an ingredient's stack-sensitive
// test (a custom kind, a data check) is the engine's and is not replayed here, so a recipe
// whose ingredient would refuse the concrete stack may still be named.
use std::sync::RwLock;

#[derive(Clone, Debug, PartialEq)]
pub struct CookingFacts {
    /// The recipe type, such as `minecraft:smelting`.
    pub kind: String,
    /// The item ids the recipe's ingredient names.
    pub inputs: Vec<String>,
    /// The result's item id.
    pub result: String,
    /// The result's count for one input item.
    pub result_count: u32,
    /// The experience the recipe grants per item cooked.
    pub experience: f32,
    /// The cooking time in ticks.
    pub time: u32,
}

static COOKING: RwLock<Vec<CookingFacts>> = RwLock::new(Vec::new());

pub fn publish(recipes: Vec<CookingFacts>) {
    if let Ok(mut table) = COOKING.write() { *table = recipes; }
}

/// The first published recipe of `kind` whose ingredient names `item`; None when the book has none.
pub fn cook(kind: &str, item: &str) -> Option<CookingFacts> {
    COOKING.read().ok()?.iter().find(|recipe| recipe.kind == kind && recipe.inputs.iter().any(|input| input == item)).cloned()
}

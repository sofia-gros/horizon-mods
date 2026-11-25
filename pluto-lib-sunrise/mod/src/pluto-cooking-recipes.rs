// The game's own cooking recipes (the furnace family: smelting, blasting, smoking and campfire
// cooking), kept published from the recipe table the game is currently using, so a mod can ask
// what a furnace does to an item (autosmelt's drop conversion, a frypan's cooking) without the
// recipe engine. The table
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

static PUBLISHED_FROM: RwLock<Option<(usize, usize)>> = RwLock::new(None);

/// Publish `recipes`' cooking list when it is not the table the last refresh published from.
/// The game replaces its recipe table whole: the startup load builds one, a world with data
/// packs builds another at open, `/reload` builds another, and a server's recipes arrive as
/// another still; in every case the new table is fully built before it takes the old one's
/// place, so the cooking list's buffer and length identify the table in use (a replacement's
/// buffer is allocated while the current table is still alive, and never equals it). Called
/// once a tick from the creature tick with the table `Game::recipes` resolves to, so the
/// published list follows whichever table the game's own furnaces would use.
pub fn refresh(recipes: &crate::crafting::Recipes) {
    let key = (recipes.cooking.as_ptr() as usize, recipes.cooking.len());
    if let Ok(slot) = PUBLISHED_FROM.read() {
        if *slot == Some(key) {
            return;
        }
    }
    publish(recipes.cooking.iter().map(|cooking| CookingFacts {
        kind: cooking.kind.clone(),
        inputs: cooking.ingredient.items.iter().cloned().collect(),
        result: cooking.result.item.to_string(),
        result_count: cooking.result.count,
        experience: cooking.experience,
        time: cooking.time,
    }).collect());
    if let Ok(mut slot) = PUBLISHED_FROM.write() {
        *slot = Some(key);
    }
}

/// The first published recipe of `kind` whose ingredient names `item`; None when the book has none.
pub fn cook(kind: &str, item: &str) -> Option<CookingFacts> {
    COOKING.read().ok()?.iter().find(|recipe| recipe.kind == kind && recipe.inputs.iter().any(|input| input == item)).cloned()
}

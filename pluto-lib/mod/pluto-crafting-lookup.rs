// The game's own crafting book (the crafting recipe type), kept published from the recipe
// table the game is currently using, so a mod can ask what a grid crafts — the match, the
// assembled result and the remaining items — without the recipe engine. The published copy
// is a snapshot of the whole table: the book's own find assembles through the recipe
// context, which borrows the table, so no borrowed answer can outlive the call; the
// snapshot's answers are owned. Lookups run the book's own code on the snapshot, so
// admission, assembly and special-marker evaluation apply exactly as at the game's table.
use std::sync::RwLock;

use super::Recipes;
use crate::items::ItemStack;

static PUBLISHED: RwLock<Option<Recipes>> = RwLock::new(None);

static PUBLISHED_FROM: RwLock<Option<(usize, usize)>> = RwLock::new(None);

impl Recipes {
    /// This table, owned: every list and map cloned, the lazily built cooking index left
    /// empty (it rebuilds on first use; crafting lookups never consult it).
    fn snapshot(&self) -> Recipes {
        Recipes {
            crafting: self.crafting.clone(),
            cooking: self.cooking.clone(),
            custom: self.custom.clone(),
            meta: self.meta.clone(),
            facts: self.facts.clone(),
            curses: self.curses.clone(),
            cook_index: std::sync::OnceLock::new(),
        }
    }
}

/// Publish a snapshot of `recipes` when it is not the table the last refresh published
/// from. The game replaces its recipe table whole: the startup load builds one, a world
/// with data packs builds another at open, `/reload` builds another, and a server's
/// recipes arrive as another still; in every case the new table is fully built before it
/// takes the old one's place, so the crafting list's buffer and length identify the table
/// in use (a replacement's buffer is allocated while the current table is still alive,
/// and never equals it). Called once a tick from the creature tick with the table
/// `Game::recipes` resolves to, so the published book follows whichever table the game's
/// own crafting would use.
pub fn refresh(recipes: &Recipes) {
    let key = (recipes.crafting.as_ptr() as usize, recipes.crafting.len());
    if let Ok(slot) = PUBLISHED_FROM.read() {
        if *slot == Some(key) {
            return;
        }
    }
    if let Ok(mut published) = PUBLISHED.write() {
        *published = Some(recipes.snapshot());
    }
    if let Ok(mut slot) = PUBLISHED_FROM.write() {
        *slot = Some(key);
    }
}

/// The published book's match for a grid of `size` rows and columns (the cells row-major,
/// `size * size` of them): the matched recipe's table name, the assembled result and the
/// remaining items placed on the full grid. None when the book matches nothing, or when
/// no table has been published yet.
pub fn craft(
    grid: &[Option<ItemStack>],
    size: usize,
) -> Option<(String, ItemStack, Vec<Option<ItemStack>>)> {
    let published = PUBLISHED.read().ok()?;
    let recipes = published.as_ref()?;
    let (name, result) = recipes.find(grid, size)?;
    let remainders = recipes.remainders(name, grid, size);
    Some((name.to_owned(), result, remainders))
}

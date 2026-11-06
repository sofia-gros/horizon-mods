// Explicit staging of ingredient alternatives on an owned native book value.
use super::{Book, Display};
use crate::items::ItemStack;
use pluto_inventory_api::recipe_book::presentation::{
    self, EntryFacts, Kind, Presentation, Rejected, Replacement,
};

fn facts(book: &Book, index: usize) -> Option<EntryFacts<'_>> {
    let entry = book.entries.get(index)?;
    let (ingredient_count, kind) = match &entry.display {
        Display::Shaped { cells, .. } => (cells.iter().filter(|cell| cell.is_some()).count(), Kind::Crafting),
        Display::Shapeless(ingredients) => (ingredients.len(), Kind::Crafting),
        Display::Furnace(_) => (1, Kind::Furnace),
    };
    Some(EntryFacts { index, recipe_id: &entry.id, display_id: entry.display_id,
        ingredient_count, kind })
}
struct NativePresentation<'a>(&'a mut Book);
impl Presentation for NativePresentation<'_> {
    type Stack = ItemStack;
    fn entry(&self, index: usize) -> Option<EntryFacts<'_>> {
        facts(self.0, index)
    }
    fn alternatives(&self, index: usize) -> Option<&[(usize, Vec<ItemStack>)]> {
        Some(&self.0.entries.get(index)?.stacks)
    }
    fn replace(&mut self, replacements: Vec<Replacement<ItemStack>>) -> Result<(), Rejected<ItemStack>> {
        if let Err(error) = presentation::validate(&replacements, |i| facts(self.0, i)) {
            return Err(Rejected { error, replacements });
        }
        // Every recoverable check has finished. Move full native values without
        // cloning opaque data, touching fields or taking an encode/decode route.
        for replacement in replacements {
            self.0.entries[replacement.entry.index].stacks = replacement.alternatives;
        }
        Ok(())
    }
}

/// The caller owns this detached value and its eventual publication. No Game
/// cache is opened, invalidated or retained. Each replace batch validates before
/// mutation; the entire callback is not a rollback transaction.
pub fn stage<R>(
    mut book: Book,
    run: impl FnOnce(&mut dyn Presentation<Stack = ItemStack>) -> R,
) -> (Book, R) {
    let result = run(&mut NativePresentation(&mut book));
    (book, result)
}

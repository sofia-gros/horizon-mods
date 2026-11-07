// Explicit ghost preview only; source recipe eligibility stays in the consumer.
use super::Game;
use horizon_client::{
    crafting::{Ingredient, Shape},
    items::ItemStack,
    pluto_inventory::recipe_book::preview::{BookKind, EntryFacts, Error, Preview, Selection},
    pluto_stack_storage::same_item_snapshot,
    recipe_book::{BookType, Category, Display, Entry},
};

pub(crate) struct NativeSelection {
    facts: EntryFacts,
    index: usize,
    entry: Entry,
}
impl Selection for NativeSelection {
    fn facts(&self) -> &EntryFacts {
        &self.facts
    }
}
pub(crate) struct GamePreview<'a, 'g>(&'a mut Game<'g>);
fn native(kind: BookKind) -> BookType {
    match kind {
        BookKind::Crafting => BookType::Crafting,
        BookKind::Furnace => BookType::Furnace,
        BookKind::BlastFurnace => BookType::BlastFurnace,
        BookKind::Smoker => BookType::Smoker,
    }
}
fn category_kind(category: Category) -> BookKind {
    match category {
        Category::CraftingBuilding
        | Category::CraftingRedstone
        | Category::CraftingEquipment
        | Category::CraftingMisc => BookKind::Crafting,
        Category::FurnaceFood | Category::FurnaceBlocks | Category::FurnaceMisc => {
            BookKind::Furnace
        }
        Category::BlastBlocks | Category::BlastMisc => BookKind::BlastFurnace,
        Category::SmokerFood => BookKind::Smoker,
    }
}
fn same_ingredient(left: &Ingredient, right: &Ingredient) -> bool {
    let mut pending = vec![(left, right)];
    while let Some((left, right)) = pending.pop() {
        if left.items != right.items || left.order != right.order || left.custom != right.custom {
            return false;
        }
        match (&left.children, &right.children) {
            (None, None) => {}
            (Some((lk, lc)), Some((rk, rc))) => {
                let same_kind = matches!(
                    (lk, rk),
                    (Shape::Compound, Shape::Compound)
                        | (Shape::Intersection, Shape::Intersection)
                        | (Shape::Difference, Shape::Difference)
                );
                if !same_kind || lc.len() != rc.len() {
                    return false;
                }
                pending.extend(lc.iter().zip(rc));
            }
            _ => return false,
        }
    }
    true
}
fn same_display(left: &Display, right: &Display) -> bool {
    match (left, right) {
        (
            Display::Shaped {
                width: lw,
                height: lh,
                cells: lc,
            },
            Display::Shaped {
                width: rw,
                height: rh,
                cells: rc,
            },
        ) => {
            lw == rw
                && lh == rh
                && lc.len() == rc.len()
                && lc.iter().zip(rc).all(|(l, r)| match (l, r) {
                    (None, None) => true,
                    (Some(l), Some(r)) => same_ingredient(l, r),
                    _ => false,
                })
        }
        (Display::Shapeless(l), Display::Shapeless(r)) => {
            l.len() == r.len() && l.iter().zip(r).all(|(l, r)| same_ingredient(l, r))
        }
        (Display::Furnace(l), Display::Furnace(r)) => same_ingredient(l, r),
        _ => false,
    }
}
fn same_stacks(left: &[ItemStack], right: &[ItemStack]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(l, r)| same_item_snapshot(l, r))
}
fn same_entry(left: &Entry, right: &Entry) -> bool {
    left.id == right.id
        && left.display_id == right.display_id
        && left.category == right.category
        && left.group == right.group
        && same_display(&left.display, &right.display)
        && same_item_snapshot(&left.result, &right.result)
        && same_stacks(&left.variants, &right.variants)
        && left.stacks.len() == right.stacks.len()
        && left
            .stacks
            .iter()
            .zip(&right.stacks)
            .all(|((li, ls), (ri, rs))| li == ri && same_stacks(ls, rs))
}
impl GamePreview<'_, '_> {
    fn entry(&self, kind: BookKind, index: usize) -> Result<&Entry, Error> {
        let game = &*self.0;
        if game.remote.is_some() {
            return Err(Error::RemoteClient);
        }
        if game.active_book() != Some(native(kind)) {
            return Err(Error::InactiveBook);
        }
        let book = game.recipe_ui.book.as_ref().ok_or(Error::MissingBook)?;
        let entry = book.entries.get(index).ok_or(Error::MissingEntry)?;
        if category_kind(entry.category) != kind {
            return Err(Error::WrongBook);
        }
        if entry.display_id < 0 {
            return Err(Error::UnregisteredDisplay);
        }
        if !entry.fits(game.craft_size, game.craft_size) {
            return Err(Error::DoesNotFit);
        }
        Ok(entry)
    }
}
impl Preview for GamePreview<'_, '_> {
    type Selection = NativeSelection;
    fn select(&self, kind: BookKind, entry_index: usize) -> Result<NativeSelection, Error> {
        let entry = self.entry(kind, entry_index)?.clone();
        Ok(NativeSelection {
            facts: EntryFacts {
                recipe_id: entry.id.clone(),
                display_id: entry.display_id,
                book: kind,
                ingredient_count: entry.requirements().len(),
                has_ingredient_previews: !entry.stacks.is_empty(),
            },
            index: entry_index,
            entry,
        })
    }
    fn show(&mut self, selected: &NativeSelection) -> Result<(), Error> {
        let current = self.entry(selected.facts.book, selected.index)?;
        if !same_entry(current, &selected.entry) {
            return Err(Error::StaleSelection);
        }
        self.0
            .fill_ghost(native(selected.facts.book), &selected.entry);
        Ok(())
    }
}
/// No automatic hook, placement interception, registry or retained actor.
pub(crate) fn with_game<R>(
    game: &mut Game<'_>,
    run: impl FnOnce(&mut GamePreview<'_, '_>) -> R,
) -> R {
    run(&mut GamePreview(game))
}

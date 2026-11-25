// Explicit read-only overlay projection. No callbacks or automatic UI hooks.
use super::{Book, Display, Entry};
use crate::items::ItemStack;
use pluto_inventory_api::recipe_book::preview::{ingredient_previews, OverlayError};

/// Uses native layout/fallback ordering and existing ingredient preview stacks.
/// This does not alter Book::placements or declare ID-only placement safe.
pub fn cells(book: &Book, entry: &Entry) -> Result<Vec<(i32, i32, Vec<ItemStack>)>, OverlayError> {
    if !entry.fits(3, 3) {
        return Err(OverlayError::DoesNotFit);
    }
    let fallback = entry
        .requirements()
        .iter()
        .map(|ingredient| {
            book.items_of(ingredient)
                .iter()
                .map(|id| ItemStack::new(id, 1))
                .collect()
        })
        .collect::<Vec<Vec<_>>>();
    let previews = ingredient_previews(&fallback, &entry.stacks)?;
    let positions = match &entry.display {
        Display::Furnace(_) => vec![(Some(0), 4)],
        _ => entry.grid_cells(3, 3),
    };
    Ok(positions
        .into_iter()
        .filter_map(|(ingredient, cell)| {
            let items = previews.get(ingredient?)?;
            (!items.is_empty()).then(|| (cell as i32 % 3, cell as i32 / 3, items.clone()))
        })
        .collect())
}

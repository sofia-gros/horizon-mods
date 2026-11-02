fn tconstruct_crafting_recipe_guard(
    game: &mut crate::game::Game<'_>,
    book_type: horizon_client::recipe_book::BookType,
    entry_index: usize,
    use_max: bool,
) -> std::ops::ControlFlow<()> {
    use horizon_items::items::ItemStack;
    use std::ops::ControlFlow::{Break, Continue};
    let Some(pos) = crate::game::tconstruct_crafting_station::position(game) else {
        return Continue(());
    };
    if &*game
        .res
        .facts()
        .get(game.world.block(pos.0, pos.1, pos.2))
        .name
        != "tconstruct:crafting_station"
        || !matches!(game.world.block_entities.get(&pos), Some(horizon_client::containers::BlockEntity::Chest(items)) if items.len() == 9)
    {
        return Break(());
    }
    let Some(entry) = game
        .recipe_ui
        .book
        .as_ref()
        .and_then(|book| book.entries.get(entry_index))
        .cloned()
    else {
        return Break(());
    };
    crate::game::tconstruct_crafting_station::refresh(game);
    let creative = game.creative;
    let mut post = None;
    let mut grid: Option<Vec<Option<ItemStack>>> = None;
    game.menu_session(|menu, env, to_menu| {
        // Reject a non-crafting menu before any item movement.
        if (0..9).any(|i| to_menu(100 + i) != Some(1 + i)) || menu.slot_count() != 46 {
            return Some(false);
        }
        if env
            .recipes
            .placement(&entry.id)
            .is_none_or(|placement| placement.impossible())
        {
            return Some(false);
        }
        let before: Vec<_> = (1..46)
            .filter_map(|slot| menu.slot(slot).cloned())
            .collect();
        let outcome = menu.place_recipe(env, &entry.id, use_max, creative, None);
        let after: Vec<_> = (1..46)
            .filter_map(|slot| menu.slot(slot).cloned())
            .collect();
        // A recipe-book operation does not craft an item. Refuse an unexpected
        // change or a drop before menu_session commits inventory changes.
        let amount = |items: &[ItemStack], item: &ItemStack| -> u64 {
            items
                .iter()
                .filter(|other| other.same_item_same_components(item))
                .map(|other| u64::from(other.count))
                .sum()
        };
        if !menu.dropped.is_empty()
            || before
                .iter()
                .any(|item| amount(&before, item) != amount(&after, item))
            || after
                .iter()
                .any(|item| amount(&before, item) != amount(&after, item))
        {
            return Some(false);
        }
        grid = Some((1..10).map(|slot| menu.slot(slot).cloned()).collect());
        post = Some(outcome);
        Some(true)
    });
    if let Some(grid) = grid {
        // Persist the actual transaction output; the temporary Game grid may
        // be refreshed by other UI code during menu processing.
        game.craft_grid = grid;
        crate::game::tconstruct_crafting_station::sync(game);
        if post == Some(horizon_client::menu::place::PostPlace::Ghost) {
            game.fill_ghost(book_type, &entry);
        }
        game.recipe_book_changed();
    }
    Break(())
}

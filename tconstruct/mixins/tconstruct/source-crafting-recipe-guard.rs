fn tconstruct_crafting_recipe_guard(
    game: &mut crate::game::Game<'_>,
    book_type: horizon_client::recipe_book::BookType,
    entry_index: usize,
    use_max: bool,
) -> std::ops::ControlFlow<()> {
    let Some(pos) = crate::game::tconstruct_crafting_station::position(game) else {
        return std::ops::ControlFlow::Continue(());
    };
    // Refuse the transaction unless its authoritative destination can be
    // refreshed and written back. The block may disappear while the UI is open.
    if &*game
        .res
        .facts()
        .get(game.world.block(pos.0, pos.1, pos.2))
        .name
        != "tconstruct:crafting_station"
        || !matches!(game.world.block_entities.get(&pos),
            Some(horizon_client::containers::BlockEntity::Chest(items)) if items.len() == 9)
    {
        return std::ops::ControlFlow::Break(());
    }
    let Some(entry) = game
        .recipe_ui
        .book
        .as_ref()
        .and_then(|book| book.entries.get(entry_index))
        .cloned()
    else {
        return std::ops::ControlFlow::Break(());
    };
    // The block entity is authoritative if another viewer changed the grid.
    crate::game::tconstruct_crafting_station::refresh(game);
    let creative = game.creative;
    let mut post = None;
    game.menu_session(|menu, env, _| {
        if env
            .recipes
            .placement(&entry.id)
            .is_none_or(|placement| placement.impossible())
        {
            return Some(false);
        }
        post = Some(menu.place_recipe(env, &entry.id, use_max, creative, None));
        Some(true)
    });
    crate::game::tconstruct_crafting_station::sync(game);
    if post == Some(horizon_client::menu::place::PostPlace::Ghost) {
        game.fill_ghost(book_type, &entry);
    }
    game.recipe_book_changed();
    std::ops::ControlFlow::Break(())
}

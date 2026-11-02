fn tconstruct_crafting_recipe_guard(
    game: &mut crate::game::Game<'_>,
    book_type: horizon_client::recipe_book::BookType,
    entry_index: usize,
    use_max: bool,
) -> std::ops::ControlFlow<()> {
    if crate::game::tconstruct_crafting_station::position(game).is_none() {
        return std::ops::ControlFlow::Continue(());
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

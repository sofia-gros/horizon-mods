fn tconstruct_crafting_recipe_guard(
    game: &mut crate::game::Game<'_>,
    book_type: horizon_client::recipe_book::BookType,
    entry_index: usize,
    use_max: bool,
) -> std::ops::ControlFlow<()> {
    if crate::game::tconstruct_crafting_station::position(game).is_some() {
        game.message = Some(("Recipe-book placement is unavailable at the Crafting Station while its item conservation issue is investigated".into(), std::time::Instant::now()));
        return std::ops::ControlFlow::Break(());
    }
    let _ = (book_type, entry_index, use_max);
    std::ops::ControlFlow::Continue(())
}

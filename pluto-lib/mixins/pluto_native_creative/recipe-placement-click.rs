fn pluto_recipe_placement_click(
    game: &mut Game<'_>,
    book_type: BookType,
    entry_index: usize,
) -> std::ops::ControlFlow<()> {
    crate::game::recipe_book::pluto_placement::attempt(game, book_type, entry_index)
}

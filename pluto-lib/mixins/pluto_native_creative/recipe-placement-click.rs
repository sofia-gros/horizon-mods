fn pluto_recipe_placement_click(
    game: &mut Game<'_>,
    book_type: BookType,
    entry_index: usize,
    use_max: bool,
) -> std::ops::ControlFlow<()> {
    crate::game::recipe_book::pluto_transactions::attempt(game, book_type, entry_index, use_max)
}

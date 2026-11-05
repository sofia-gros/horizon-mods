fn tconstruct_fake_material_book_preview(
    game: &mut crate::game::Game<'_>, book_type: BookType, entry_index: usize,
) -> std::ops::ControlFlow<()> {
    if game.tconstruct_preview_fake_material_recipe(book_type, entry_index) {
        std::ops::ControlFlow::Break(())
    } else { std::ops::ControlFlow::Continue(()) }
}

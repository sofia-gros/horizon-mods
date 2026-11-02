fn tconstruct_materials_book_turn(game: &mut Game<'_>, forward: bool) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(BookScreen::View { pages, page, lectern: None }) = game.book.as_mut() else { return Continue(()); };
    if pages.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") { return Continue(()); }
    *page = tconstruct_materials_book_spread::turn(*page, pages.len(), forward);
    *game.world.chunks.__horizon_mixins_tconstruct_materials_book_history_mut() = None;
    game.audio.play("minecraft:item.book.page_turn", None, 1.0, 1.0);
    Break(())
}

fn tconstruct_materials_book_turn(game: &mut Game<'_>, forward: bool) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(BookScreen::View { pages, page, lectern: None }) = game.book.as_mut() else { return Continue(()); };
    if pages.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") { return Continue(()); }
    let base = (*page / 2) * 2;
    *page = if forward { if base + 2 < pages.len() { base + 2 } else { base } } else { base.saturating_sub(2) };
    game.audio.play("minecraft:item.book.page_turn", None, 1.0, 1.0);
    Break(())
}

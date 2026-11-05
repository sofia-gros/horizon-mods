fn tconstruct_materials_book_key(game: &mut Game<'_>, key: Option<KeyCode>) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(BookScreen::View { pages, lectern: None, .. }) = game.book.as_ref() else { return Continue(()); };
    if pages.first().and_then(|page| page.spans.first()).and_then(|span| span.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") { return Continue(()); }
    let forward = match key {
        Some(KeyCode::ArrowLeft | KeyCode::KeyA) => false,
        Some(KeyCode::ArrowRight | KeyCode::KeyD) => true,
        _ => return Continue(()),
    };
    // Reuse the existing guide spread boundaries, link-history reset and audio.
    game.turn_page(forward);
    Break(())
}

fn tconstruct_materials_book_key(game: &mut Game<'_>, key: Option<KeyCode>) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(BookScreen::View { pages, lectern: None, .. }) = game.book.as_ref() else { return Continue(()); };
    if pages.first().and_then(|page| page.spans.first()).and_then(|span| span.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") { return Continue(()); }
    let forward = match key {
        Some(KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::PageUp) => false,
        Some(KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::PageDown) => true,
        _ => return Continue(()),
    };
    // Consume these keys once, using guide policy without altering native turns.
    tconstruct_materials_book_navigation::turn(game, forward);
    Break(())
}

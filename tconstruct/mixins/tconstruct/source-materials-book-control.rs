fn tconstruct_materials_book_control(game: &mut Game<'_>, action: Option<&str>) -> std::ops::ControlFlow<horizon_client::Result<serde_json::Value>> {
    use std::ops::ControlFlow::{Break, Continue};
    let forward = match action {
        Some("next") => true,
        Some("back") => false,
        _ => return Continue(()),
    };
    if !tconstruct_materials_book_navigation::turn(game, forward) { return Continue(()); }
    // Native book_control has the same post-action readout. Avoid re-entering
    // book_control or turn_page: the latter has ordinary one-page semantics and
    // cannot admit this guide's usize::MAX cover sentinel.
    let Some(BookScreen::View { pages, page, lectern: None }) = game.book.as_ref() else { unreachable!("synchronous guide turn preserves its view"); };
    Break(Ok(serde_json::json!({ "page": page, "pages": pages.len(), "lectern": null })))
}

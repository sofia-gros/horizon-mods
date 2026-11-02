fn tconstruct_materials_book_response(game: &mut Game<'_>, packet: &C) -> std::ops::ControlFlow<Result<()>> {
    use std::ops::ControlFlow::{Break, Continue};
    let C::CustomPayload(message) = packet else { return Continue(()); };
    if message.channel != "tconstruct:materials_book" { return Continue(()); }
    let Some(super::books::BookScreen::View { pages, page, lectern: None }) = game.book.as_mut() else { return Break(Ok(())); };
    if pages.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") || message.data.len() > 262_144 { return Break(Ok(())); }
    let Some(decoded) = super::books::tconstruct_materials_book_wire::decode(&message.data) else { return Break(Ok(())); };
    if *page != usize::MAX { *page = (*page).min(decoded.len() - 1); }
    *pages = decoded;
    Break(Ok(()))
}

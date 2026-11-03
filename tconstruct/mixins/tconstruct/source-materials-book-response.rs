fn tconstruct_materials_book_response(game: &mut Game<'_>, packet: &C) -> std::ops::ControlFlow<Result<()>> {
    use std::ops::ControlFlow::{Break, Continue};
    let C::CustomPayload(message) = packet else { return Continue(()); };
    if message.channel != "tconstruct:materials_book" { return Continue(()); }
    let sequence = *game.world.chunks.__horizon_mixins_tconstruct_materials_book_request_sequence();
    let Some(super::books::BookScreen::View { pages, page, lectern: None }) = game.book.as_mut() else { return Break(Ok(())); };
    let Some(decoded) = super::books::tconstruct_materials_book_wire::decode_reply(sequence, pages, &message.data) else { return Break(Ok(())); };
    if *page != usize::MAX { *page = (*page).min(decoded.len() - 1); }
    *pages = decoded;
    Break(Ok(()))
}

// Non-cancellable HEAD at handle_packet. Registration only, no reply application.
fn tconstruct_materials_book_response_bootstrap(game: &mut Game<'_>, packet: &C) {
    let C::CustomPayload(message) = packet else { return; };
    if message.channel != "tconstruct:materials_book" { return; }
    let _ = super::books::tconstruct_materials_book_response::ensure_registered(game);
}

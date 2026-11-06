// TConstruct cover/spread/history policy, called synchronously by this guide's
// existing input handlers and the native book-control command adapter. This is
// not an owner token, screen lifetime proof or PlutoLib's one-page PageTurn.
use super::*;

pub(super) fn turn(game: &mut Game<'_>, forward: bool) -> bool {
    let Some(BookScreen::View { pages, page, lectern: None }) = game.book.as_mut() else { return false; };
    if pages.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") { return false; }
    *page = tconstruct_materials_book_spread::turn(*page, pages.len(), forward);
    *game.world.chunks.__horizon_mixins_tconstruct_materials_book_history_mut() = None;
    game.audio.play("minecraft:item.book.page_turn", None, 1.0, 1.0);
    true
}

fn tconstruct_materials_book_click(game: &mut Game<'_>, size: [u32; 2]) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(BookScreen::View { pages, page, lectern: None }) = game.book.as_ref() else { return Continue(()); };
    if pages.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") { return Continue(()); }
    let batch = HudBatch::new(size);
    let center = batch.width / 2.0;
    let top = (batch.height / 2.0 - 100.0).floor();
    let cover = *page == usize::MAX;
    let base = if cover { 0 } else { (*page / 2) * 2 };
    let count = pages.len();
    let previous = *page;
    let (mx, my) = game.cursor;
    let inside = |x: f32, y: f32, w: f32, h: f32| mx >= x && my >= y && mx < x + w && my < y + h;
    let mut link = None;
    for offset in 0..if cover { 0 } else { 2 } {
        let Some(text) = pages.get(base + offset) else { continue; };
        let origin = if offset == 0 { center - 206.0 + 16.0 } else { center + 8.0 };
        let row = ((my - (top + 12.0)) / 9.0).floor();
        if row < 0.0 || row >= 17.0 || mx < origin { continue; }
        let lines = game.gui.atlas.font.split(text, 182.0);
        let Some(line) = lines.get(row as usize) else { continue; };
        let line = line.resolve_legacy();
        let mut x = mx - origin;
        for (c, style) in line.chars() {
            x -= game.gui.atlas.font.advance(c, style);
            if x < 0.0 { link = style.click.clone(); break; }
        }
        if link.is_some() { break; }
    }
    if let Some(link) = link {
        if let horizon_client::text::ClickEvent::ChangePage(target) = &link {
            tconstruct_materials_book_spread::remember(game.world.chunks.__horizon_mixins_tconstruct_materials_book_history_mut(), previous, *target, count);
        }
        game.book_follow_link(link, count, None);
    }
    else if (cover || base + 2 < count) && inside(center + if cover { 80.0 } else { 165.0 }, top + 175.0, 18.0, 10.0) { game.turn_page(true); }
    else if !cover && inside(center - 184.0, top + 175.0, 18.0, 10.0) { game.turn_page(false); }
    else if !cover && inside(center - 215.0, top, 18.0, 18.0) {
        *game.world.chunks.__horizon_mixins_tconstruct_materials_book_history_mut() = None;
        if let Some(BookScreen::View { page, .. }) = game.book.as_mut() { *page = 0; }
    }
    else if inside(center - 9.0, top + 193.0, 18.0, 10.0) {
        if let Some(old) = game.world.chunks.__horizon_mixins_tconstruct_materials_book_history_mut().take() {
            if let Some(BookScreen::View { page, .. }) = game.book.as_mut() { *page = old; }
        }
    }
    Break(())
}

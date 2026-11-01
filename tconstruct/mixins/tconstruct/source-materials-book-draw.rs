fn tconstruct_draw_materials_book(game: &Game<'_>, batch: &mut HudBatch) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(BookScreen::View { pages, page, lectern: None }) = game.book.as_ref() else { return Continue(()); };
    let Some(marker) = pages.get(*page).and_then(|p| p.spans.first())
        .and_then(|span| span.style.insertion.as_deref()).and_then(|s| s.strip_prefix("tconstruct:book:")) else { return Continue(()); };
    let Ok(spec) = serde_json::from_str::<serde_json::Value>(marker) else { return Continue(()); };
    let atlas = &game.gui.atlas;
    let left = ((batch.width - 192.0) / 2.0).floor();
    batch.fill(atlas, 0.0, 0.0, batch.width, batch.height, rgba(16, 16, 16, 160));
    if let Some(mut rect) = atlas.sprites.get("gui/book").copied() {
        let (du, dv) = ((rect.u1 - rect.u0) / rect.w as f32, (rect.v1 - rect.v0) / rect.h as f32);
        rect.u1 = rect.u0 + 192.0 * du;
        rect.v1 = rect.v0 + 192.0 * dv;
        batch.rect_uv(left, 2.0, 192.0, 192.0, rect, u32::MAX);
    }
    let caption = format!("Page {} of {}", page + 1, pages.len());
    batch.text(atlas, &caption, left + 148.0 - HudBatch::text_width(atlas, &caption), 18.0, BLACK, false);
    if let Some(text) = pages.get(*page) {
        for (i, line) in game.page_lines(text).iter().take(LINES).enumerate() {
            batch.text_styled(atlas, line, left + 36.0, 32.0 + i as f32 * 9.0, BLACK, false);
        }
    }
    let (mx, my) = game.cursor;
    for (show, x, name) in [(*page + 1 < pages.len(), 116.0, "page_forward"), (*page > 0, 43.0, "page_backward")] {
        if show {
            let hover = mx >= left + x && mx < left + x + 23.0 && (159.0..172.0).contains(&my);
            let sprite = format!("gui/sprites/widget/{}{}", name, if hover { "_highlighted" } else { "" });
            batch.sprite(atlas, &sprite, left + x, 159.0);
        }
    }
    let button_x = batch.width / 2.0 - 100.0;
    let hover = mx >= button_x && mx < button_x + 200.0 && (196.0..216.0).contains(&my);
    ui::button(batch, atlas, button_x, 196.0, 200.0, 20.0, &Text::literal(game.translate("gui.done", "Done")), true, hover);
    (|| {
    if let Some(id) = spec.get("image").and_then(|v| v.as_str()) {
        let Some(name) = id.strip_prefix("tconstruct:book/images/").and_then(|s| s.strip_suffix(".png")) else { return; };
        if !name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_/-".contains(&c)) { return; }
        let sprite = format!("gui/tconstruct_book_{}", name.replace(['/', '-'], "_"));
        if let Some(rect) = atlas.sprites.get(&sprite) {
            let scale = (110.0 / rect.w as f32).min(105.0 / rect.h as f32).min(1.0);
            let (w, h) = (rect.w as f32 * scale, rect.h as f32 * scale);
            batch.sprite_scaled(atlas, &sprite, left + 38.0 + (110.0 - w) / 2.0, 51.0, w, h, u32::MAX);
        }
        return;
    }
    let Some(slots) = spec.get("slots").and_then(|v| v.as_array()) else { return; };
    let ox = left + 38.0;
    let oy = 55.0;
    let grid = if spec.get("grid").and_then(|v| v.as_u64()) == Some(2) { 2 } else { 3 };
    for (i, item) in slots.iter().take(9).enumerate() {
        if i % 3 >= grid || i / 3 >= grid { continue; }
        let x = ox + (i % 3) as f32 * 18.0;
        let y = oy + (i / 3) as f32 * 18.0;
        batch.fill(atlas, x, y, 17.0, 17.0, rgba(120, 90, 50, 100));
        if let Some(item) = item.as_str() { game.draw_item(batch, &ItemStack::new(item, 1), x, y); }
    }
    if let Some(id) = spec.get("result").and_then(|v| v.as_str()) {
        batch.text(atlas, "→", ox + 57.0, oy + 18.0, BLACK, false);
        game.draw_item(batch, &ItemStack::new(id, 1), ox + 78.0, oy + 18.0);
    }
    })();
    Break(())
}

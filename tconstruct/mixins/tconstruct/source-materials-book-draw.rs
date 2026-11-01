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
    let draw_stack = |batch: &mut HudBatch, id: &str, count: u32, damage: u32, maximum: Option<u32>, x: f32, y: f32| {
        let mut stack = ItemStack::new(id, count);
        stack.damage = damage;
        if id.starts_with("tconstruct:") {
            use horizon_world::nbt::Tag;
            let materials = ["head", "handle", "extra", "large", "extra_large"].into_iter()
                .map(|kind| Tag::String(format!("tconstruct:ui_render#{kind}").as_str().into())).collect();
            stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
                ("tic_display", Tag::Byte(1)),
                ("tic_materials", Tag::List(materials)),
                ("Material", Tag::String("tconstruct:ui_render#head".into())),
            ])));
        }
        if let Some(maximum) = maximum.and_then(|n| i32::try_from(n).ok()).filter(|n| *n > 0) {
            stack.data_mut().set_other("minecraft:max_damage", Some(horizon_world::nbt::Tag::Int(maximum)));
        }
        game.draw_item(batch, &stack, x, y);
    };
    (|| {
    if let Some(id) = spec.get("image").and_then(|v| v.as_str()) {
        let Some(name) = id.strip_prefix("tconstruct:book/images/").and_then(|s| s.strip_suffix(".png")) else { return; };
        if !name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_/-".contains(&c)) { return; }
        let sprite = format!("gui/tconstruct_book_{}", name.replace(['/', '-'], "_"));
        if let Some(rect) = atlas.sprites.get(&sprite) {
            let width = spec.get("width").and_then(|v| v.as_u64()).unwrap_or(rect.w as u64).clamp(1, 4096) as f32;
            let height = spec.get("height").and_then(|v| v.as_u64()).unwrap_or(rect.h as u64).clamp(1, 4096) as f32;
            let scale = (110.0 / width).min(105.0 / height).min(1.0);
            let (w, h) = (width * scale, height * scale);
            batch.sprite_scaled(atlas, &sprite, left + 38.0 + (110.0 - w) / 2.0, 51.0, w, h, u32::MAX);
        }
        return;
    }
    if let Some(item) = spec.get("item").and_then(|v| v.as_str()) {
        draw_stack(batch, item, 1, 0, None, left + 87.0, 72.0);
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
        let chosen = item.as_array().and_then(|items| (!items.is_empty()).then(|| &items[(game.ticks / 20) as usize % items.len()])).unwrap_or(item);
        let id = chosen.as_str().or_else(|| chosen.get("id").and_then(|v| v.as_str()));
        let damage = chosen.get("damage").and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok()).unwrap_or(0);
        let maximum = chosen.get("max_damage").and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok());
        if let Some(id) = id { draw_stack(batch, id, 1, damage, maximum, x, y); }
    }
    if let Some(result) = spec.get("result") {
        let id = result.as_str().or_else(|| result.get("id").and_then(|v| v.as_str()));
        if let Some(id) = id {
            batch.text(atlas, "→", ox + 57.0, oy + 18.0, BLACK, false);
            let count = spec.get("count").and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok()).filter(|n| *n > 0).unwrap_or(1);
            let damage = result.get("damage").and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok()).unwrap_or(0);
            let maximum = result.get("max_damage").and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok());
            draw_stack(batch, id, count, damage, maximum, ox + 78.0, oy + 18.0);
        }
    }
    })();
    Break(())
}

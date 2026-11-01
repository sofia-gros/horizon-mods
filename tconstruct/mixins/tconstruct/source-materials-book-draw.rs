fn tconstruct_draw_materials_book(game: &Game<'_>, batch: &mut HudBatch) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(BookScreen::View { pages, page, lectern: None }) = game.book.as_ref() else { return Continue(()); };
    if pages.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") { return Continue(()); }
    let atlas = &game.gui.atlas;
    let center = batch.width / 2.0;
    let top = (batch.height / 2.0 - 100.0).floor();
    let cover = *page == usize::MAX;
    let base = if cover { 0 } else { (*page / 2) * 2 };
    batch.fill(atlas, 0.0, 0.0, batch.width, batch.height, rgba(16, 16, 16, 160));
    if cover {
        if let Some(mut rect) = atlas.sprites.get("gui/tconstruct_book_covers_materials_and_you_cover").copied() {
            let (du, dv) = ((rect.u1 - rect.u0) / rect.w as f32, (rect.v1 - rect.v0) / rect.h as f32);
            rect.u1 = rect.u0 + 206.0 * du; rect.v1 = rect.v0 + 200.0 * dv;
            batch.rect_uv(center - 103.0, top, 206.0, 200.0, rect, u32::MAX);
        }
    } else if let Some(rect) = atlas.sprites.get("gui/tconstruct_book_covers_materials_and_you_pages").copied() {
        let (du, dv) = ((rect.u1 - rect.u0) / rect.w as f32, (rect.v1 - rect.v0) / rect.h as f32);
        let mut under = rect;
        under.u1 = under.u0 + 412.0 * du; under.v1 = under.v0 + 200.0 * dv;
        batch.rect_uv(center - 206.0, top, 412.0, 200.0, under, u32::MAX);
        let mut paper = rect;
        paper.u1 = paper.u0 + 412.0 * du; paper.v0 += 200.0 * dv; paper.v1 = paper.v0 + 200.0 * dv;
        batch.rect_uv(center - 206.0, top, 412.0, 200.0, paper, u32::MAX);
    }
    let (mx, my) = game.cursor;
    if let Some(rect) = atlas.sprites.get("gui/tconstruct_book_covers_materials_and_you_pages").copied() {
        let (du, dv) = ((rect.u1 - rect.u0) / rect.w as f32, (rect.v1 - rect.v0) / rect.h as f32);
        for (show, x, sy, w, h) in [
            (cover || base + 2 < pages.len(), center + if cover { 80.0 } else { 165.0 }, 0.0, 18.0, 10.0),
            (!cover, center - 184.0, 10.0, 18.0, 10.0),
            (!cover, center - 215.0, 40.0, 18.0, 18.0),
        ] {
            if show {
                let mut arrow = rect;
                arrow.u0 = rect.u0 + 412.0 * du; arrow.u1 = arrow.u0 + w * du;
                arrow.v0 = rect.v0 + sy * dv; arrow.v1 = arrow.v0 + h * dv;
                let y = if sy == 40.0 { top } else { top + 175.0 };
                batch.rect_uv(x, y, w, h, arrow, u32::MAX);
            }
        }
    }
    let button_x = center - 100.0;
    let button_y = (top + 202.0).min(batch.height - 20.0);
    let hover = mx >= button_x && mx < button_x + 200.0 && my >= button_y && my < button_y + 20.0;
    ui::button(batch, atlas, button_x, button_y, 200.0, 20.0, &Text::literal(game.translate("gui.done", "Done")), true, hover);
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
    for offset in 0..if cover { 0 } else { 2 } {
    let Some(text) = pages.get(base + offset) else { continue; };
    let left = center - 206.0 + offset as f32 * 206.0 + 16.0 - 36.0;
    let is_graphic = text.spans.first().and_then(|span| span.style.insertion.as_deref()).is_some_and(|id| id.starts_with("tconstruct:book:"));
    for (i, line) in game.gui.atlas.font.split(text, 182.0).iter().take(LINES).enumerate() {
        let x = if is_graphic && i == 0 { left + 36.0 + (182.0 - HudBatch::text_width(atlas, &text.spans.first().map(|s| s.text.as_str()).unwrap_or(""))) / 2.0 } else { left + 36.0 };
        batch.text_styled(atlas, line, x, top + 12.0 + i as f32 * 9.0, BLACK, false);
    }
    let number = (base + offset + 1).to_string();
    batch.text(atlas, &number, left + 36.0 + (182.0 - HudBatch::text_width(atlas, &number)) / 2.0, top + 178.0, rgba(170, 170, 170, 255), false);
    let Some(spec) = text.spans.first().and_then(|s| s.style.insertion.as_deref()).and_then(|s| s.strip_prefix("tconstruct:book:")).and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) else { continue; };
    (|| {
    if let Some(id) = spec.get("image").and_then(|v| v.as_str()) {
        let Some(name) = id.strip_prefix("tconstruct:book/images/").and_then(|s| s.strip_suffix(".png")) else { return; };
        if !name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_/-".contains(&c)) { return; }
        let sprite = format!("gui/tconstruct_book_{}", name.replace(['/', '-'], "_"));
        if let Some(rect) = atlas.sprites.get(&sprite) {
            let width = spec.get("width").and_then(|v| v.as_u64()).unwrap_or(rect.w as u64).clamp(1, 4096) as f32;
            let height = spec.get("height").and_then(|v| v.as_u64()).unwrap_or(rect.h as u64).clamp(1, 4096) as f32;
            let scale = (182.0 / width).min(100.0 / height).min(1.0);
            let (w, h) = (width * scale, height * scale);
            batch.sprite_scaled(atlas, &sprite, left + 36.0 + (182.0 - w) / 2.0, top + 31.0, w, h, u32::MAX);
            if let Some(body) = spec.get("body").and_then(|v| v.as_str()) {
                let content = Text::literal(body);
                for (i, line) in atlas.font.split(&content, 182.0).iter().enumerate() {
                    batch.text_styled(atlas, line, left + 36.0, top + 36.0 + h + i as f32 * 9.0, BLACK, false);
                }
            }
        }
        return;
    }
    if let Some(item) = spec.get("item").and_then(|v| v.as_str()) {
        draw_stack(batch, item, 1, 0, None, left + 87.0, top + 52.0);
        return;
    }
    let Some(slots) = spec.get("slots").and_then(|v| v.as_array()) else { return; };
    let grid = if spec.get("grid").and_then(|v| v.as_u64()) == Some(2) { 2 } else { 3 };
    let frame_width = if grid == 2 { 155.0 } else { 183.0 };
    let frame_height = if grid == 2 { 78.0 } else { 114.0 };
    let ox = left + 36.0 + (182.0 - frame_width) / 2.0;
    let oy = top + 31.0;
    if let Some(mut rect) = atlas.sprites.get("gui/tconstruct_book_mantle_crafting").copied() {
        let (du, dv) = ((rect.u1 - rect.u0) / rect.w as f32, (rect.v1 - rect.v0) / rect.h as f32);
        rect.u1 = rect.u0 + frame_width * du;
        if grid == 2 { rect.v0 += 114.0 * dv; }
        rect.v1 = rect.v0 + frame_height * dv;
        batch.rect_uv(ox, oy, frame_width, frame_height, rect, rgba(229, 198, 130, 255));
    }
    // Book screens pause the local world. Cycle on real elapsed time so tag
    // candidates still change while the reader is open.
    let cycle = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |time| time.as_secs() as usize);
    for (i, item) in slots.iter().take(9).enumerate() {
        if i % 3 >= grid || i / 3 >= grid { continue; }
        let x = ox + 5.0 + (i % 3) as f32 * 36.0;
        let y = oy + 5.0 + (i / 3) as f32 * 36.0;
        let chosen = item.as_array().and_then(|items| (!items.is_empty()).then(|| &items[cycle % items.len()])).unwrap_or(item);
        let id = chosen.as_str().or_else(|| chosen.get("id").and_then(|v| v.as_str()));
        let damage = chosen.get("damage").and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok()).unwrap_or(0);
        let maximum = chosen.get("max_damage").and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok());
        if let Some(id) = id {
            let start = batch.vertices.len();
            draw_stack(batch, id, 1, damage, maximum, x, y);
            batch.scale_since(start, x, y, 2.0, 2.0);
        }
    }
    if let Some(result) = spec.get("result") {
        let id = result.as_str().or_else(|| result.get("id").and_then(|v| v.as_str()));
        if let Some(id) = id {
            let count = spec.get("count").and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok()).filter(|n| *n > 0).unwrap_or(1);
            let damage = result.get("damage").and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok()).unwrap_or(0);
            let maximum = result.get("max_damage").and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok());
            let x = ox + if grid == 2 { 118.0 } else { 146.0 };
            let y = oy + if grid == 2 { 23.0 } else { 41.0 };
            let start = batch.vertices.len();
            draw_stack(batch, id, count, damage, maximum, x, y);
            batch.scale_since(start, x, y, 2.0, 2.0);
        }
    }
    })();
    }
    Break(())
}

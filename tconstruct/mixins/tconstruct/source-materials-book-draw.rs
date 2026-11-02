fn tconstruct_draw_materials_book(game: &Game<'_>, batch: &mut HudBatch) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(BookScreen::View { pages, page, lectern: None }) = game.book.as_ref() else { return Continue(()); };
    if pages.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") { return Continue(()); }
    let atlas = &game.gui.atlas;
    let puny = pages.first().and_then(|p| p.spans.first()).is_some_and(|s| s.text.starts_with("Puny Smelting\n"));
    let cover_sprite = if puny { "gui/tconstruct_book_covers_puny_smelting_cover" } else { "gui/tconstruct_book_covers_materials_and_you_cover" };
    let pages_sprite = if puny { "gui/tconstruct_book_covers_puny_smelting_pages" } else { "gui/tconstruct_book_covers_materials_and_you_pages" };
    let center = batch.width / 2.0;
    let top = (batch.height / 2.0 - 100.0).floor();
    let cover = *page == usize::MAX;
    let base = if cover { 0 } else { (*page / 2) * 2 };
    batch.fill(atlas, 0.0, 0.0, batch.width, batch.height, rgba(16, 16, 16, 160));
    if cover {
        if let Some(mut rect) = atlas.sprites.get(cover_sprite).copied() {
            let (du, dv) = ((rect.u1 - rect.u0) / rect.w as f32, (rect.v1 - rect.v0) / rect.h as f32);
            rect.u1 = rect.u0 + 206.0 * du; rect.v1 = rect.v0 + 200.0 * dv;
            batch.rect_uv(center - 103.0, top, 206.0, 200.0, rect, u32::MAX);
        }
    } else if let Some(rect) = atlas.sprites.get(pages_sprite).copied() {
        let (du, dv) = ((rect.u1 - rect.u0) / rect.w as f32, (rect.v1 - rect.v0) / rect.h as f32);
        let mut under = rect;
        under.u1 = under.u0 + 412.0 * du; under.v1 = under.v0 + 200.0 * dv;
        batch.rect_uv(center - 206.0, top, 412.0, 200.0, under, u32::MAX);
        let mut paper = rect;
        paper.u1 = paper.u0 + 412.0 * du; paper.v0 += 200.0 * dv; paper.v1 = paper.v0 + 200.0 * dv;
        batch.rect_uv(center - 206.0, top, 412.0, 200.0, paper, u32::MAX);
    }
    if let Some(rect) = atlas.sprites.get(pages_sprite).copied() {
        let (du, dv) = ((rect.u1 - rect.u0) / rect.w as f32, (rect.v1 - rect.v0) / rect.h as f32);
        for (show, x, sy, w, h) in [
            (cover || base + 2 < pages.len(), center + if cover { 80.0 } else { 165.0 }, 0.0, 18.0, 10.0),
            (!cover, center - 184.0, 10.0, 18.0, 10.0),
            (!cover, center - 215.0, 40.0, 18.0, 18.0),
            (game.world.chunks.__horizon_mixins_tconstruct_materials_book_history().is_some(), center - 9.0, 30.0, 18.0, 10.0),
        ] {
            if show {
                let mut arrow = rect;
                arrow.u0 = rect.u0 + 412.0 * du; arrow.u1 = arrow.u0 + w * du;
                arrow.v0 = rect.v0 + sy * dv; arrow.v1 = arrow.v0 + h * dv;
                let y = if sy == 40.0 { top } else if sy == 30.0 { top + 193.0 } else { top + 175.0 };
                batch.rect_uv(x, y, w, h, arrow, u32::MAX);
            }
        }
    }
    let mut tooltip: Option<Vec<Text>> = None;
    let mut item_tooltip: Option<ItemStack> = None;
    let (mx, my) = game.cursor;
    let hovered = |x: f32, y: f32, size: f32| mx >= x && mx < x + size && my >= y && my < y + size;
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
        stack
    };
    for offset in 0..if cover { 0 } else { 2 } {
    let Some(text) = pages.get(base + offset) else { continue; };
    let left = if offset == 0 { center - 206.0 + 16.0 - 36.0 } else { center + 8.0 - 36.0 };
    let is_title = text.spans.first().and_then(|span| span.style.insertion.as_deref()).is_some_and(|id| id.starts_with("tconstruct:book:") || id == "tconstruct:book-title");
    for (i, line) in game.gui.atlas.font.split(text, 182.0).iter().take(17).enumerate() {
        let x = if is_title && i == 0 { left + 36.0 + (182.0 - HudBatch::text_width(atlas, &text.spans.first().map(|s| s.text.as_str()).unwrap_or(""))) / 2.0 } else { left + 36.0 };
        let material_title = i == 0 && text.spans.first().and_then(|span| span.style.insertion.as_deref()).is_some_and(|id| id.starts_with("tconstruct:book:") && id.contains("\"material_panel\""));
        batch.text_styled(atlas, line, x, top + 12.0 + i as f32 * 9.0, BLACK, material_title);
    }
    let number = (base + offset + 1).to_string();
    batch.text(atlas, &number, left + 36.0 + (182.0 - HudBatch::text_width(atlas, &number)) / 2.0, top + 178.0, rgba(170, 170, 170, 255), false);
    let Some(spec) = text.spans.first().and_then(|s| s.style.insertion.as_deref()).and_then(|s| s.strip_prefix("tconstruct:book:")).and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) else { continue; };
    (|| {
    if let Some(panel) = spec.get("material_panel") {
        let Some(material) = panel.get("material").and_then(|v| v.as_str()) else { return; };
        let page_x = left + 36.0;
        let sidebar_x = page_x + if offset == 0 { 0.0 } else { 164.0 };
        let stat_x = page_x + if offset == 0 { 24.0 } else { 2.0 };
        let cycle = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |time| time.as_secs() as usize);
        if let Some(sidebar) = panel.get("sidebar").and_then(|v| v.as_array()) {
            for (i, entry) in sidebar.iter().take(9).enumerate() {
                let id = entry.get("item").and_then(|v| v.as_str()).or_else(|| entry.get("choices").and_then(|v| v.as_array()).filter(|v| !v.is_empty()).and_then(|v| v[cycle % v.len()].as_str()));
                let Some(id) = id else { continue; };
                let y = top + 23.0 + i as f32 * 16.0;
                let mut stack = ItemStack::new(id, 1);
                if let Some(material) = entry.get("material").and_then(|v| v.as_str()) {
                    stack.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([("Material", horizon_world::nbt::Tag::String(material.into()))])));
                } else if let Some(materials) = entry.get("materials").and_then(|v| v.as_array()) {
                    let ids: Vec<_> = materials.iter().filter_map(|v| v.as_str()).map(|id| horizon_world::nbt::Tag::String(id.into())).collect();
                    stack.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([("tic_materials", horizon_world::nbt::Tag::List(ids))])));
                } else if entry.get("display_tool").and_then(|v| v.as_bool()) == Some(true) {
                    let ids: Vec<_> = ["head", "handle", "extra", "large", "extra_large"].into_iter().map(|kind| horizon_world::nbt::Tag::String(format!("tconstruct:ui_render#{kind}").as_str().into())).collect();
                    stack.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([("tic_display", horizon_world::nbt::Tag::Byte(1)), ("tic_materials", horizon_world::nbt::Tag::List(ids))])));
                }
                game.draw_item(batch, &stack, sidebar_x, y);
                if hovered(sidebar_x, y, 16.0) {
                    if let Some(lines) = entry.get("tooltip").and_then(|v| v.as_array()) {
                        tooltip = Some(lines.iter().filter_map(|v| v.as_str()).filter(|v| !v.is_empty()).map(Text::literal).collect());
                    } else { item_tooltip = Some(stack); }
                }
            }
        }
        let Some(groups) = panel.get("groups").and_then(|v| v.as_array()) else { return; };
        let mut row_y = top + 28.0;
        for row in 0..2 {
            let mut max_lines = None;
            for col in 0..2 {
                let Some(group) = groups.get(row * 2 + col).filter(|v| v.is_object()) else { continue; };
                let x = stat_x - 3.0 + col as f32 * 81.0;
                let lines = group.get("lines").and_then(|v| v.as_array()).map(Vec::as_slice).unwrap_or(&[]);
                max_lines = Some(max_lines.unwrap_or(0usize).max(lines.len()));
                let mut title_x = x;
                if let Some(parts) = group.get("parts").and_then(|v| v.as_array()).filter(|v| !v.is_empty()) {
                    if let Some(id) = parts[cycle % parts.len()].as_str() {
                        let mut stack = ItemStack::new(id, 1);
                        stack.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([("Material", horizon_world::nbt::Tag::String(material.into()))])));
                        let start = batch.vertices.len();
                        game.draw_item(batch, &stack, x, row_y + 1.0);
                        batch.scale_since(start, x, row_y + 1.0, 0.5, 0.5);
                        if hovered(x, row_y + 1.0, 8.0) { item_tooltip = Some(stack); }
                        title_x += 10.0;
                    }
                }
                if let Some(name) = group.get("title").and_then(|v| v.as_str()) {
                    let title = Text::styled(name, horizon_client::text::Style { bold:Some(true), underlined:Some(true), ..Default::default() });
                    batch.text_styled(atlas, &title, title_x, row_y, BLACK, false);
                }
                for (i, line) in lines.iter().take(5).enumerate() {
                    let y = row_y + 12.0 + i as f32 * 10.0;
                    if let Some(text) = line.get("runs").and_then(tconstruct_materials_book_layout::decode_runs) {
                        batch.text_styled(atlas, &text, x, y, BLACK, false);
                    }
                    if mx >= x && mx < x + 81.0 && my >= y && my < y + 9.0 {
                        if let Some(description) = line.get("tooltip").and_then(|v| v.as_str()).filter(|v| !v.is_empty()) {
                            tooltip = Some(vec![Text::literal(description)]);
                        }
                    }
                }
            }
            if let Some(lines) = max_lines { row_y += 15.0 + lines as f32 * 10.0; }
        }
        if let Some(flavor) = panel.get("flavor_runs").and_then(tconstruct_materials_book_layout::decode_runs) {
            for (i, line) in atlas.font.split(&flavor, 155.0).iter().enumerate() {
                batch.text_styled(atlas, line, stat_x - 3.0, row_y + 5.0 + i as f32 * 9.0, BLACK, false);
            }
        }
        return;
    }
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
            if let Some(content) = spec.get("body_runs").and_then(tconstruct_materials_book_layout::decode_runs) {
                for (i, line) in atlas.font.split(&content, 182.0).iter().enumerate() {
                    batch.text_styled(atlas, line, left + 36.0, top + 36.0 + h + i as f32 * 9.0, BLACK, false);
                }
            }
        }
        return;
    }
    if let Some(id) = spec.get("showcase_item").and_then(|v| v.as_str()) {
        let (x, y) = (left + 36.0 + 83.0, top + 43.0);
        let stack = draw_stack(batch, id, 1, 0, None, x, y);
        if hovered(x, y, 16.0) { item_tooltip = Some(stack); }
        if let Some(content) = spec.get("body_runs").and_then(tconstruct_materials_book_layout::decode_runs) {
            for (i, line) in atlas.font.split(&content, 182.0).iter().enumerate() {
                batch.text_styled(atlas, line, left + 36.0, top + 76.0 + i as f32 * 9.0, BLACK, false);
            }
        }
        return;
    }
    if let Some(icon) = spec.get("source_icon").and_then(|v| v.as_str()).filter(|v| matches!(*v, "slotless" | "upgrade" | "ability" | "defense")) {
        let sprite = format!("gui/tconstruct_book_slot_{icon}");
        let (x, y) = (left + 36.0 + 71.0, top + 31.0);
        batch.sprite_scaled(atlas, &sprite, x, y, 40.0, 40.0, u32::MAX);
        if mx >= x && mx < x + 40.0 && my >= y && my < y + 40.0 {
            tooltip = spec.get("tooltip").and_then(|v| v.as_array()).map(|lines| lines.iter().take(16)
                .filter_map(|v| v.as_str().filter(|s| s.len() <= 256)).map(Text::literal).collect());
        }
        if let Some(content) = spec.get("body_runs").and_then(tconstruct_materials_book_layout::decode_runs) {
            for (i, line) in atlas.font.split(&content, 182.0).iter().enumerate() {
                batch.text_styled(atlas, line, left + 36.0, top + 76.0 + i as f32 * 9.0, BLACK, false);
            }
        }
        return;
    }
    if let Some(item) = spec.get("item").and_then(|v| v.as_str()) {
        if let Some(text) = spec.get("description_runs").and_then(tconstruct_materials_book_layout::decode_runs) {
            for (i, line) in atlas.font.split(&text, 172.0).iter().take(6).enumerate() {
                batch.text_styled(atlas, line, left + 41.0, top + 31.0 + i as f32 * 9.0, BLACK, false);
            }
        }
        if let Some(properties) = spec.get("properties").and_then(|v| v.as_str()).filter(|v| !v.is_empty()) {
            let title = Text::styled("Properties", horizon_client::text::Style { underlined: Some(true), ..Default::default() });
            batch.text_styled(atlas, &title, left + 41.0, top + 94.0, BLACK, false);
            let text = Text::literal(properties);
            for (i, line) in atlas.font.split(&text, 86.0).iter().take(7).enumerate() {
                batch.text_styled(atlas, line, left + 41.0, top + 106.0 + i as f32 * 9.0, BLACK, false);
            }
        }
        let inputs = spec.get("inputs").and_then(|v| v.as_array()).map(Vec::as_slice).unwrap_or(&[]);
        let count = inputs.len().min(9);
        let (sx, sy, sw, sh, cols) = match count {
            0 => (0.0, 75.0, 0.0, 0.0, 1), 1 => (0.0, 75.0, 22.0, 22.0, 1),
            2 => (0.0, 97.0, 40.0, 22.0, 2), 3 => (0.0, 119.0, 58.0, 22.0, 3),
            4 => (0.0, 141.0, 40.0, 40.0, 2), 5 => (0.0, 181.0, 58.0, 41.0, 3),
            6 => (58.0, 133.0, 58.0, 40.0, 3), _ => (58.0, 173.0, 58.0, 58.0, 3),
        };
        let x = left + 36.0 + 140.0 - sw / 2.0;
        let y = top + 12.0 + 138.0 - sh / 2.0;
        if let Some(rect) = atlas.sprites.get("gui/tconstruct_book_modify").copied() {
            let (du, dv) = ((rect.u1 - rect.u0) / rect.w as f32, (rect.v1 - rect.v0) / rect.h as f32);
            let mut region = |dx: f32, dy: f32, u: f32, v: f32, w: f32, h: f32, tint: u32| {
                let mut r = rect;
                r.u0 = rect.u0 + u * du; r.u1 = r.u0 + w * du;
                r.v0 = rect.v0 + v * dv; r.v1 = r.v0 + h * dv;
                batch.rect_uv(dx, dy, w, h, r, tint);
            };
            region(x + (sw - 42.0) / 2.0, y + 28.0, 214.0, 0.0, 42.0, 46.0, u32::MAX);
            if count > 0 { region(x, y, sx, sy, sw, sh, rgba(229, 198, 130, 255)); }
            region(x + (sw - 22.0) / 2.0, y - 27.0, 0.0, 75.0, 22.0, 22.0, u32::MAX);
        }
        let (demo_x, demo_y) = (x + (sw - 16.0) / 2.0, y - 24.0);
        let demo = draw_stack(batch, item, 1, 0, None, demo_x, demo_y);
        if hovered(demo_x, demo_y, 16.0) { item_tooltip = Some(demo); }
        let cycle = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |time| time.as_secs() as usize);
        for (i, input) in inputs.iter().take(9).enumerate() {
            let (dx, dy) = if count == 5 && i >= 3 { (12.0 + (i - 3) as f32 * 18.0, 22.0) }
                else { (3.0 + (i % cols) as f32 * 18.0, match i / cols { 0 => 3.0, 1 => 22.0, _ => 40.0 }) };
            if let Some(id) = input.get("part").and_then(|v| v.as_str()) {
                let mut stack = ItemStack::new(id, 1);
                let kind = ["head", "handle", "extra", "large", "extra_large"][i % 5];
                stack.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([
                    ("Material", horizon_world::nbt::Tag::String(format!("tconstruct:ui_render#{kind}").as_str().into()))
                ])));
                game.draw_item(batch, &stack, x + dx, y + dy);
                if hovered(x + dx, y + dy, 16.0) { item_tooltip = Some(stack); }
            } else if let Some(choices) = input.get("choices").and_then(|v| v.as_array()).filter(|v| !v.is_empty()) {
                if let Some(id) = choices[cycle % choices.len()].as_str() {
                    let stack = draw_stack(batch, id, 1, 0, None, x + dx, y + dy);
                    if hovered(x + dx, y + dy, 16.0) { item_tooltip = Some(stack); }
                }
            }
        }
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
    if let Some(content) = spec.get("body_runs").and_then(tconstruct_materials_book_layout::decode_runs) {
        for (i, line) in atlas.font.split(&content, 182.0).iter().enumerate() {
            batch.text_styled(atlas, line, left + 36.0, oy + frame_height + 5.0 + i as f32 * 9.0, BLACK, false);
        }
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
        let result = result.as_array().and_then(|items| (!items.is_empty()).then(|| &items[cycle % items.len()])).unwrap_or(result);
        let id = result.as_str().or_else(|| result.get("id").and_then(|v| v.as_str()));
        if let Some(id) = id {
            let count = result.get("count").or_else(|| spec.get("count")).and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok()).filter(|n| *n > 0).unwrap_or(1);
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
    if let Some(stack) = item_tooltip {
        game.draw_tooltip_lines(batch, &game.stack_tooltip(&stack), mx, my);
    } else if let Some(lines) = tooltip.filter(|lines| !lines.is_empty()) {
        let lines: Vec<_> = lines.iter().flat_map(|line| atlas.font.split(line, 200.0)).collect();
        ui::tooltip(batch, atlas, &lines, (mx, my));
    }
    Break(())
}

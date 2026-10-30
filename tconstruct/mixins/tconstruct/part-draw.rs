fn draw_tconstruct_station(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) -> std::ops::ControlFlow<()> {
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return std::ops::ControlFlow::Continue(()); };
    let atlas = &game.gui.atlas;
    if !station && game.container_size() > 27 {
        let atlas = &game.gui.atlas;
        let count = if game.remote.is_some() {
            game.container.and_then(|key| game.world.chunks.__horizon_mixins_tconstruct_builder_side_size().get(&key).copied()).unwrap_or(game.container_size() - 27)
        } else { game.container_size() - 27 };
        let columns = if count > 27 { 6 } else { 3 };
        let left = if columns == 6 { -176.0 } else { -122.0 };
        let width = columns as f32 * 18.0;
        let height = count.div_ceil(columns) as f32 * 18.0;
        if let Some(sprite) = atlas.sprites.get("gui/tconstruct_generic").copied() {
            let region = |batch: &mut horizon_client::hud::HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
                let mut r = sprite;
                let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
                r.u0 += sx * du; r.v0 += sy * dv;
                r.u1 = r.u0 + sw * du; r.v1 = r.v0 + sh * dv;
                batch.rect_uv(ox + left + x, oy + y, w, h, r, u32::MAX);
            };
            for (sx, sy, x, y) in [(0.0, 0.0, 0.0, 0.0), (57.0, 0.0, width + 7.0, 0.0),
                (0.0, 57.0, 0.0, height + 7.0), (57.0, 57.0, width + 7.0, height + 7.0)] {
                region(batch, sx, sy, 7.0, 7.0, x, y, 7.0, 7.0);
            }
            region(batch, 7.0, 0.0, 50.0, 7.0, 7.0, 0.0, width, 7.0);
            region(batch, 7.0, 57.0, 50.0, 7.0, 7.0, height + 7.0, width, 7.0);
            region(batch, 0.0, 7.0, 7.0, 50.0, 0.0, 7.0, 7.0, height);
            region(batch, 57.0, 7.0, 7.0, 50.0, width + 7.0, 7.0, 7.0, height);
            for i in 0..count {
                region(batch, 7.0, 7.0, 18.0, 18.0,
                    7.0 + (i % columns) as f32 * 18.0, 7.0 + (i / columns) as f32 * 18.0, 18.0, 18.0);
            }
        }
    }
    let texture = if station { "gui/tconstruct_tinker_station" } else { "gui/tconstruct_part_builder" };
    batch.sprite_region(atlas, texture, [ox, oy, 176.0, 184.0], 256.0, 256.0);
    // The atlas contains reusable elements outside the 176x184 background.
    let part = |batch: &mut horizon_client::hud::HudBatch, sx: f32, sy: f32, x: f32, y: f32, color: u32| {
        if let Some(mut r) = atlas.sprites.get(texture).copied() {
            let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
            r.u0 += sx * du; r.v0 += sy * dv;
            r.u1 = r.u0 + 18.0 * du; r.v1 = r.v0 + 18.0 * dv;
            batch.rect_uv(ox + x, oy + y, 18.0, 18.0, r, color);
        }
    };
    let (title_key, fallback) = if station { ("gui.tconstruct.tinker_station", "Tinker Station") } else { ("gui.tconstruct.part_builder", "Part Builder") };
    let title = game.pack.names.display_get(title_key).unwrap_or_else(|| fallback.to_owned());
    let inventory = game.pack.names.display_get("container.inventory").unwrap_or_else(|| "Inventory".to_owned());
    batch.text(atlas, &title, ox + 8.0, oy + 6.0, horizon_client::hud::rgba(64, 64, 64, 255), false);
    batch.text(atlas, &inventory, ox + 8.0, oy + 90.0, horizon_client::hud::rgba(64, 64, 64, 255), false);
    if station {
        if crate::game::tconstruct_name_ui::available(game) {
            let mut edit = game.container.and_then(|key| game.world.chunks.__horizon_mixins_tconstruct_station_name_edit().get(&key)).cloned()
                .unwrap_or_else(|| crate::game::tconstruct_name_ui::new_box(ox, oy));
            edit.x = ox + 80.0; edit.y = oy + 7.0;
            edit.draw(batch, atlas, None);
        }
        let active_layout = crate::game::tconstruct_ui::station_layout(game);
        for (index, layout) in crate::game::tconstruct_ui::station_layouts(game).iter().enumerate() {
            let Some((x, y)) = crate::game::tconstruct_source_station_layout::button_position(index) else { break; };
            let (x, y) = (x as f32, y as f32);
            let selected = active_layout.as_ref().is_some_and(|active| active.id == layout.id);
            let hovered = (ox + x..ox + x + 18.0).contains(&game.cursor.0) && (oy + y..oy + y + 18.0).contains(&game.cursor.1);
            if let Some(mut r) = atlas.sprites.get("gui/tconstruct_icons").copied() {
                let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
                r.u0 += (if selected { 144.0 } else if hovered { 216.0 } else { 180.0 }) * du;
                r.v0 += 216.0 * dv;
                r.u1 = r.u0 + 18.0 * du; r.v1 = r.v0 + 18.0 * dv;
                batch.rect_uv(ox + x, oy + y, 18.0, 18.0, r, u32::MAX);
            }
            if let Some(item) = layout.icon.get("item").and_then(serde_json::Value::as_str) {
                game.draw_item(batch, &horizon_items::items::ItemStack::new(item, 1), ox + x + 1.0, oy + y + 1.0);
            } else if let Some(pattern) = layout.icon.get("pattern").and_then(serde_json::Value::as_str) {
                if let Some(("tconstruct", name)) = pattern.split_once(':') {
                    batch.sprite(atlas, &format!("gui/tconstruct_patterns/{name}"), ox + x + 1.0, oy + y + 1.0);
                }
            }
            let label = game.pack.names.display_get(&layout.translation).unwrap_or_else(|| layout.translation.clone());
            horizon_client::widgets::target([ox + x, oy + y, 18.0, 18.0]);
            horizon_client::widgets::record(&label, [ox + x, oy + y, 18.0, 18.0], true);
        }
        // Match the background to the same source positions used for clicks.
        let positions = active_layout.as_ref().map(|layout| {
            let slots: Vec<_> = if let Some(tool) = layout.tool_slot.as_ref() { std::iter::once(tool).chain(layout.input_slots.iter()).collect() }
                else { layout.input_slots.iter().collect() };
            slots.into_iter().map(|slot| (slot.x as f32 - 1.0, slot.y as f32 - 1.0)).collect::<Vec<_>>()
        })
            .unwrap_or_else(|| vec![(52.0, 21.0), (14.0, 59.0), (32.0, 41.0)]);
        for (x, y) in positions {
            part(batch, 176.0, 0.0, x, y, horizon_client::hud::rgba(255, 255, 255, 71));
            part(batch, 194.0, 0.0, x, y, u32::MAX);
        }
        if let Some(horizon_client::containers::BlockEntity::Chest(slots)) = game.container_kind() {
            for (x, action) in [(114.0, 24), (134.0, 25)] {
                let output = if game.remote.is_some() {
                    slots.get(action as usize).cloned().flatten()
                } else if action == 24 {
                    crate::game::tconstruct_source_station_world::building_plan(&game.world, slots).map(|(_, output)| output)
                } else { crate::game::tconstruct_source_station_world::operation_plan(&game.world, slots).map(|plan| plan.output) };
                if let Some(stack) = output {
                    if action != 24 || crate::game::tconstruct_ui::station_accepts_output(game, &stack) {
                        game.draw_item(batch, &stack, ox + x, oy + 38.0);
                    }
                }
            }
        }
    } else {
        if let Some(horizon_client::containers::BlockEntity::Chest(slots)) = game.container_kind() {
            if let Some(icon) = atlas.sprites.get("gui/tconstruct_icons").copied() {
                let (du, dv) = ((icon.u1 - icon.u0) / icon.w as f32, (icon.v1 - icon.v0) / icon.h as f32);
                for (slot, sx, sy, x) in [(0, 0.0, 216.0, 7.0), (1, 54.0, 234.0, 28.0)] {
                    if slots.get(slot).and_then(Option::as_ref).is_none() {
                        let mut r = icon;
                        r.u0 += sx * du; r.v0 += sy * dv; r.u1 = r.u0 + 18.0 * du; r.v1 = r.v0 + 18.0 * dv;
                        batch.rect_uv(ox + x, oy + 42.0, 18.0, 18.0, r, u32::MAX);
                    }
                }
            }
            if let Some(key) = game.container {
                let patterns = game.world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&key)
                    .and_then(|(_, _, patterns)| patterns.as_deref()).unwrap_or(&[]);
                let progress = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll().get(&key).map_or(0.0, |state| state.0);
                let offset = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll().get(&key).map_or(0, |state| state.1);
                let selected = game.world.chunks.__horizon_mixins_tconstruct_part_selections().get(&key);
                for (relative, pattern) in patterns.iter().skip(offset).take(16).enumerate() {
                    let x = 51.0 + (relative % 4) as f32 * 18.0;
                    let y = 15.0 + (relative / 4) as f32 * 18.0;
                    let hovered = game.cursor.0 >= ox + x && game.cursor.0 < ox + x + 18.0
                        && game.cursor.1 >= oy + y && game.cursor.1 < oy + y + 18.0;
                    let v = if selected == Some(pattern) { 33.0 } else if hovered { 51.0 } else { 15.0 };
                    part(batch, 176.0, v, x, y, u32::MAX);
                    if let Some(("tconstruct", name)) = pattern.split_once(':') {
                        batch.sprite(atlas, &format!("gui/tconstruct_patterns/{name}"), ox + x + 1.0, oy + y + 1.0);
                        let key = format!("pattern.tconstruct.{}", name.replace('/', "."));
                        let label = game.pack.names.display_get(&key).unwrap_or(key);
                        horizon_client::widgets::target([ox + x, oy + y, 18.0, 18.0]);
                        horizon_client::widgets::record(&label, [ox + x, oy + y, 18.0, 18.0], true);
                    }
                }
                if let Some(mut r) = atlas.sprites.get(texture).copied() {
                    let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
                    r.u0 += if patterns.len() > 16 { 176.0 * du } else { 188.0 * du };
                    r.u1 = r.u0 + 12.0 * du; r.v1 = r.v0 + 15.0 * dv;
                    batch.rect_uv(ox + 126.0, oy + 15.0 + (59.0 * progress.clamp(0.0, 1.0)) as i32 as f32, 12.0, 15.0, r, u32::MAX);
                }
                let result = if game.remote.is_some() { None }
                    else { game.world.chunks.__horizon_mixins_tconstruct_builder_preview().get(&key).and_then(|(_, _, _, result)| result.as_ref()) };
                if let Some(item) = result { game.draw_item(batch, item, ox + 148.0, oy + 42.0); }
            }
        }
    }
    std::ops::ControlFlow::Break(())
}

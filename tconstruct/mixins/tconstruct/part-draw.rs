fn draw_tconstruct_station(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) -> std::ops::ControlFlow<()> {
    if crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) {
        crate::game::tconstruct_source_smeltery_draw::draw(game, batch, ox, oy);
        return std::ops::ControlFlow::Break(());
    }
    if crate::game::tconstruct_ui::is_melter(game) {
        let atlas = &game.gui.atlas;
        batch.sprite_region(atlas, "gui/tconstruct_melter", [ox, oy, 176.0, 166.0], 256.0, 256.0);
        let has_heater = if game.remote.is_some() {
            game.remote_chest_menu().is_some_and(|(rows, _)| rows == 6)
        } else { game.container_size() > 27 };
        if let Some(mut sprite) = atlas.sprites.get("gui/tconstruct_melter").copied() {
            let du = (sprite.u1 - sprite.u0) / sprite.w as f32;
            let dv = (sprite.v1 - sprite.v0) / sprite.h as f32;
            let (x, width, sx) = if has_heater { (150.0, 18.0, 176.0) } else { (152.0, 14.0, 194.0) };
            sprite.u0 += sx * du; sprite.v0 += 52.0 * dv;
            sprite.u1 = sprite.u0 + width * du; sprite.v1 = sprite.v0 + 36.0 * dv;
            batch.rect_uv(ox + x, oy + 31.0, width, 36.0, sprite, u32::MAX);
        }
        let title = game.pack.names.display_get("container.tconstruct.melter").unwrap_or_else(|| "Melter".to_owned());
        let inventory = game.pack.names.display_get("container.inventory").unwrap_or_else(|| "Inventory".to_owned());
        let color = horizon_client::hud::rgba(64, 64, 64, 255);
        batch.text(atlas, &title, ox + 8.0, oy + 6.0, color, false);
        batch.text(atlas, &inventory, ox + 8.0, oy + 72.0, color, false);
        // The same saved state is synchronized to guests for this menu.
        let position = if game.remote.is_some() { crate::game::remote::tconstruct_source_machine_remote::position(game) } else { game.container };
        let saved = position.and_then(|pos| crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:melter"));
        let parsed = saved.as_ref().and_then(crate::game::tconstruct_source_melter_state::Melter::from_nbt);
        if let Some(melter) = parsed {
            if let Some(fluid) = &melter.tank.fluid {
                // Tile the first frame of the original still-fluid texture in
                // the source tank's 52x52 area, scaled by the stored amount.
                let sprite_name = crate::game::tconstruct_source_melter_render::sprite(fluid);
                if let Some(sprite) = atlas.sprites.get(&sprite_name).or_else(|| atlas.sprites.get("block/tconstruct_fluid_fallback")).copied() {
                    let height = (52.0 * melter.tank.amount as f32 / melter.tank.capacity as f32).clamp(0.0, 52.0);
                    let (du, dv) = ((sprite.u1 - sprite.u0) / sprite.w as f32, (sprite.v1 - sprite.v0) / sprite.h as f32);
                    let mut y = 0.0;
                    while y < height {
                        let h = (height - y).min(16.0);
                        let mut x = 0.0;
                        while x < 52.0 {
                            let w = (52.0_f32 - x).min(16.0);
                            let mut part = sprite; part.u1 = part.u0 + w * du; part.v1 = part.v0 + h * dv;
                            batch.rect_uv(ox + 90.0 + x, oy + 68.0 - y - h, w, h, part, crate::game::tconstruct_source_melter_render::color(fluid));
                            x += w;
                        }
                        y += h;
                    }
                }
                let translation = format!("fluid.{}", fluid.replace(':', "."));
                let name = game.pack.names.display_get(&translation).unwrap_or_else(|| fluid.clone());
                batch.text(atlas, &name, ox + 90.0, oy + 24.0, color, false);
                batch.text(atlas, &format!("{} mB", melter.tank.amount), ox + 90.0, oy + 36.0, color, false);
            }
            for (slot, progress) in melter.progress.iter().enumerate() {
                if *progress != 0 {
                    let text = if *progress < 0 { "Full".to_owned() } else { progress.to_string() };
                    batch.text(atlas, &text, ox + 44.0, oy + 19.0 + slot as f32 * 18.0, color, false);
                }
            }
        }
        return std::ops::ControlFlow::Break(());
    }
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return std::ops::ControlFlow::Continue(()); };
    let atlas = &game.gui.atlas;
    if !station && game.container_size() > 27 {
        let atlas = &game.gui.atlas;
        let count = if game.remote.is_some() {
            game.container.and_then(|key| game.world.chunks.__horizon_mixins_tconstruct_builder_side_size().get(&key).copied()).unwrap_or(game.container_size() - 27)
        } else { game.container_size() - 27 };
        let columns = crate::game::tconstruct_source_side_layout::columns(count);
        let left = crate::game::tconstruct_source_side_layout::left(count);
        let scrollable = crate::game::tconstruct_source_side_layout::max_offset(count) > 0;
        let width = columns as f32 * 18.0 + if scrollable { 10.0 } else { 0.0 };
        let slots_height = crate::game::tconstruct_source_side_layout::visible_rows(count) as f32 * 18.0;
        let height = slots_height + 10.0;
        let offset = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&pos)).map_or(0, |state| state.0);
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
            region(batch, 25.0, 7.0, 18.0, 10.0, 7.0, 7.0, width, 10.0);
            let title_key = crate::game::tconstruct_ui::side_title(game);
            let title = game.pack.names.display_get(title_key).unwrap_or_else(|| "Inventory".to_owned());
            batch.text(atlas, &title, ox + left + 7.0, oy + 6.0, horizon_client::hud::rgba(64, 64, 64, 255), false);
            let first = offset.min(crate::game::tconstruct_source_side_layout::max_offset(count)) * columns;
            for i in 0..count.saturating_sub(first).min(crate::game::tconstruct_source_side_layout::visible_rows(count) * columns) {
                region(batch, 7.0, 7.0, 18.0, 18.0,
                    7.0 + (i % columns) as f32 * 18.0, 17.0 + (i / columns) as f32 * 18.0, 18.0, 18.0);
            }
            let shown = count.saturating_sub(first).min(crate::game::tconstruct_source_side_layout::visible_rows(count) * columns);
            if shown % columns != 0 {
                for i in shown..shown.div_ceil(columns) * columns {
                    region(batch, 25.0, 7.0, 18.0, 18.0,
                        7.0 + (i % columns) as f32 * 18.0, 17.0 + (i / columns) as f32 * 18.0, 18.0, 18.0);
                }
            }
            if scrollable {
                let x = 7.0 + columns as f32 * 18.0;
                region(batch, 43.0, 8.0, 12.0, 30.0, x, 17.0, 10.0, slots_height);
                let progress = offset.min(crate::game::tconstruct_source_side_layout::max_offset(count)) as f32
                    / crate::game::tconstruct_source_side_layout::max_offset(count) as f32;
                region(batch, 7.0, 25.0, 10.0, 15.0, x, 17.0 + progress * (slots_height - 15.0), 10.0, 15.0);
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
        // Source TinkerStationScreen paints the translucent item cover before
        // rendering the active layout's slot backgrounds.
        if let Some(mut r) = atlas.sprites.get(texture).copied() {
            let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
            r.u0 += 176.0 * du; r.v0 += 18.0 * dv;
            r.u1 = r.u0 + 70.0 * du; r.v1 = r.v0 + 64.0 * dv;
            batch.rect_uv(ox + 7.0, oy + 18.0, 70.0, 64.0, r, horizon_client::hud::rgba(255, 255, 255, 209));
        }
        if let Some(sprite) = atlas.sprites.get(texture).copied() {
            let region = |batch: &mut horizon_client::hud::HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
                let mut r = sprite;
                let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
                r.u0 += sx * du; r.v0 += sy * dv; r.u1 = r.u0 + sw * du; r.v1 = r.v0 + sh * dv;
                batch.rect_uv(ox + x, oy + y, w, h, r, u32::MAX);
            };
            // The original top beams and decorations frame the button and
            // information modules; they do not create additional slots.
            for (left, width) in [(-132.0, 128.0), (180.0, 126.0)] {
                region(batch, 0.0, 202.0, 2.0, 7.0, left, 0.0, 2.0, 7.0);
                region(batch, 2.0, 202.0, 129.0, 7.0, left + 2.0, 0.0, width, 7.0);
                region(batch, 131.0, 202.0, 2.0, 7.0, left + width + 2.0, 0.0, 2.0, 7.0);
            }
            for y in [1.0, 90.0] {
                region(batch, 0.0, 196.0, 5.0, 4.0, 187.0, y, 5.0, 4.0);
                region(batch, 9.0, 196.0, 9.0, 4.0, 294.0, y, 9.0, 4.0);
            }
            let count = crate::game::tconstruct_ui::station_layouts(game).len();
            for index in 0..count {
                let Some((x, y)) = crate::game::tconstruct_source_station_layout::button_position(index) else { break; };
                region(batch, 0.0, 198.0, 18.0, 2.0, x as f32, y as f32 - 2.0, 18.0, 2.0);
                if index + 6 < count { region(batch, 0.0, 196.0, 18.0, 2.0, x as f32, y as f32 + 18.0, 18.0, 2.0); }
            }
        }
        crate::game::tconstruct_station_info_ui::draw(game, batch, ox, oy);
        if crate::game::tconstruct_name_ui::available(game) {
            if let Some(mut r) = atlas.sprites.get(texture).copied() {
                let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
                r.v0 += 244.0 * dv; r.u1 = r.u0 + 90.0 * du; r.v1 = r.v0 + 12.0 * dv;
                batch.rect_uv(ox + 79.0, oy + 5.0, 90.0, 12.0, r, u32::MAX);
            }
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
        if let (Some(layout), Some(entity)) = (active_layout.as_ref(), game.container_kind()) {
            let specs: Vec<_> = if let Some(tool) = layout.tool_slot.as_ref() {
                std::iter::once(tool).chain(layout.input_slots.iter()).collect()
            } else { layout.input_slots.iter().collect() };
            for (index, spec) in specs.into_iter().enumerate() {
                if entity.slots().get(index).and_then(Option::as_ref).is_some() { continue; }
                if let Some(("tconstruct", name)) = spec.icon.as_deref().and_then(|icon| icon.split_once(':')) {
                    batch.sprite(atlas, &format!("gui/tconstruct_patterns/{name}"), ox + spec.x as f32, oy + spec.y as f32);
                }
            }
        }
        if let Some(horizon_client::containers::BlockEntity::Chest(slots)) = game.container_kind() {
            let action = if crate::game::tconstruct_source_station_world::has_operation_tool(slots) { 25 } else { 24 };
            for (x, action) in [(114.0, action)] {
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
        crate::game::tconstruct_builder_info_ui::draw(game, batch, ox, oy);
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

fn draw_tconstruct_station(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) -> std::ops::ControlFlow<()> {
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return std::ops::ControlFlow::Continue(()); };
    let atlas = &game.gui.atlas;
    if !station && game.container_size() > 27 {
        let atlas = &game.gui.atlas;
        let count = if game.remote.is_some() {
            game.container.and_then(|key| crate::game::tconstruct_ui::side_inventory(game, key))
                .and_then(|side| game.world.block_entities.get(&side)).map_or(game.container_size() - 27, |entity| entity.slots().len())
        } else { game.container_size() - 27 };
        batch.fill(atlas, ox - 117.0, oy + 5.0, 60.0, (count.div_ceil(3) as f32 * 18.0) + 6.0, horizon_client::hud::rgba(198, 198, 198, 255));
        for i in 0..count {
            let x = ox - 115.0 + (i % 3) as f32 * 18.0;
            let y = oy + 7.0 + (i / 3) as f32 * 18.0;
            batch.sprite(atlas, "gui/sprites/container/slot", x, y);
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
        // These three locations come from the upstream pickaxe layout. The
        // default repair layout and layout selection still need to be ported.
        for (x, y) in [(52.0, 21.0), (14.0, 59.0), (32.0, 41.0)] {
            part(batch, 176.0, 0.0, x, y, horizon_client::hud::rgba(255, 255, 255, 71));
            part(batch, 194.0, 0.0, x, y, u32::MAX);
        }
        if let Some(horizon_client::containers::BlockEntity::Chest(slots)) = game.container_kind() {
            for (x, action) in [(114.0, 24), (134.0, 25)] {
                let output = if action == 24 {
                    crate::game::tconstruct_source_station_world::building_plan(&game.world, slots).map(|(_, output)| output)
                } else { crate::game::tconstruct_source_station_world::swap_plan(&game.world, slots).or_else(|| crate::game::tconstruct_source_station_world::repair_plan(&game.world, slots)).map(|(output, _)| output) };
                if let Some(stack) = output { game.draw_item(batch, &stack, ox + x, oy + 38.0); }
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

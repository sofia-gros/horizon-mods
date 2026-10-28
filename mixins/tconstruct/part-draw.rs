fn draw_tconstruct_station(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) -> std::ops::ControlFlow<()> {
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return std::ops::ControlFlow::Continue(()); };
    let atlas = &game.gui.atlas;
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
    let title = if station { "Tinker Station" } else { "Part Builder" };
    batch.text(atlas, title, ox + 8.0, oy + 6.0, horizon_client::hud::rgba(64, 64, 64, 255), false);
    if station {
        // The original atlas uses separately drawn slot backgrounds. Clear its
        // unused fixed result/side-slot artwork before laying out our two actions.
        batch.fill(atlas, ox + 77.0, oy + 16.0, 96.0, 79.0, horizon_client::hud::rgba(198, 198, 198, 255));
        for (x, y) in [(26.0, 34.0), (47.0, 34.0), (68.0, 34.0), (113.0, 37.0), (133.0, 37.0)] {
            part(batch, 176.0, 0.0, x, y, horizon_client::hud::rgba(255, 255, 255, 71));
            part(batch, 194.0, 0.0, x, y, u32::MAX);
        }
        batch.text(atlas, "Build", ox + 109.0, oy + 22.0, horizon_client::hud::rgba(64, 64, 64, 255), false);
        batch.text(atlas, "Repair", ox + 135.0, oy + 65.0, horizon_client::hud::rgba(64, 64, 64, 255), false);
        if let Some(horizon_client::containers::BlockEntity::Chest(slots)) = game.container_kind() {
            for (x, action) in [(114.0, 24), (134.0, 25)] {
                let mut preview = slots.clone();
                let mut output = None;
                if crate::game::tconstruct_tx::take(&mut preview, &mut output, true, action) {
                    if let Some(stack) = output { game.draw_item(batch, &stack, ox + x, oy + 38.0); }
                }
            }
        }
    } else {
        for x in [51.0, 69.0, 87.0] { part(batch, 176.0, 15.0, x, 15.0, u32::MAX); }
        for (x, id) in [(52.0, "tconstruct:flint_pick_head"), (70.0, "tconstruct:wood_handle"), (88.0, "tconstruct:wood_binding")] {
            game.draw_item(batch, &horizon_client::items::ItemStack::new(id, 1), ox + x, oy + 16.0);
        }
    }
    std::ops::ControlFlow::Break(())
}

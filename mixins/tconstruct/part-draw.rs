fn draw_tconstruct_station(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) -> std::ops::ControlFlow<()> {
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return std::ops::ControlFlow::Continue(()); };
    let atlas = &game.gui.atlas;
    let texture = if station { "gui/tconstruct_tinker_station" } else { "gui/tconstruct_part_builder" };
    batch.sprite_region(atlas, texture, [ox, oy, 176.0, 184.0], 256.0, 256.0);
    let title = if station { "Tinker Station" } else { "Part Builder" };
    batch.text(atlas, title, ox + 8.0, oy + 6.0, horizon_client::hud::rgba(64, 64, 64, 255), false);
    if station {
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
        for (x, id) in [(60.0, "tconstruct:flint_pick_head"), (78.0, "tconstruct:wood_handle"), (96.0, "tconstruct:wood_binding")] {
            game.draw_item(batch, &horizon_client::items::ItemStack::new(id, 1), ox + x, oy + 18.0);
        }
    }
    std::ops::ControlFlow::Break(())
}

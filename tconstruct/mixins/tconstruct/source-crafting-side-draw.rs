fn tconstruct_crafting_side_draw(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) {
    let Some((_, count, offset, kind)) = crate::game::tconstruct_crafting_station::side(game) else { return; };
    let key = match kind { crate::game::tconstruct_chest_rules::ChestKind::Part => "gui.tconstruct.part_chest", crate::game::tconstruct_chest_rules::ChestKind::Tinkers => "gui.tconstruct.tinkers_chest" };
    let title = game.pack.names.display_get(key).unwrap_or_else(|| "Inventory".to_owned());
    crate::game::tconstruct_source_side_draw::draw(game, batch, ox, oy, count, offset, &title);
}

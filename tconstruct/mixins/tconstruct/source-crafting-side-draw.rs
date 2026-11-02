fn tconstruct_crafting_side_draw(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) {
    let Some((pos, count, offset, kind)) = crate::game::tconstruct_crafting_station::side(game) else { return; };
    let key = match kind {
        crate::game::tconstruct_chest_rules::ChestKind::Part => "gui.tconstruct.part_chest",
        crate::game::tconstruct_chest_rules::ChestKind::Tinkers => "gui.tconstruct.tinkers_chest",
        crate::game::tconstruct_chest_rules::ChestKind::Plain(_) => {
            let code=if game.remote.is_some() {game.world.chunks.__horizon_mixins_tconstruct_builder_side_title().get(&(i32::MIN,game.remote_menu_id(),0)).copied().unwrap_or(0)} else {crate::game::tconstruct_source_side_layout::title_code(&game.res.facts().get(game.world.block(pos.0,pos.1,pos.2)).name)};
            crate::game::tconstruct_source_side_layout::title_key(code)
        }
    };
    let title = game.pack.names.display_get(key).unwrap_or_else(|| "Inventory".to_owned());
    crate::game::tconstruct_source_side_draw::draw(game, batch, ox, oy, count, offset, &title);
}

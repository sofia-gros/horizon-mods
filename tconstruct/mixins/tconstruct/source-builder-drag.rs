fn builder_drag(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if crate::game::tconstruct_ui::kind(game) != Some(false) { return Continue(()); }
    let Some(pos) = game.container else { return Continue(()); };
    if !game.world.chunks.__horizon_mixins_tconstruct_builder_scroll().get(&pos).is_some_and(|s| s.2) { return Continue(()); }
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Break(()); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let (_, oy) = game.inventory_origin(&batch);
    let count = game.world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&pos)
        .and_then(|(_, _, list)| list.as_ref()).map_or(0, Vec::len);
    let state = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().get_mut(&pos).unwrap();
    state.0 = crate::game::tconstruct_source_builder_layout::drag(count, state.0, game.cursor.1 - oy);
    state.1 = crate::game::tconstruct_source_builder_layout::offset(count, state.0);
    Break(())
}

fn smeltery_release(game: &mut crate::game::Game<'_>) {
    if let Some(pos) = crate::game::tconstruct_source_smeltery_ui::position(game) {
        game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_dragging_mut().remove(&pos);
    }
}

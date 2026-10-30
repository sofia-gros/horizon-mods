fn builder_close(game: &mut crate::game::Game<'_>) {
    if crate::game::tconstruct_ui::kind(game) != Some(false) { return; }
    if let Some(pos) = game.container {
        game.world.chunks.__horizon_mixins_tconstruct_builder_side_mut().remove(&pos);
        game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().remove(&pos);
        if game.remote.is_some() {
            game.world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_cast_costs_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_cast_wire_recv_mut().remove(&pos);
        }
    }
}

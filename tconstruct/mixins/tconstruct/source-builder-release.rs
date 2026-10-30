fn builder_release(game: &mut crate::game::Game<'_>) {
    if let Some(pos) = game.container {
        if let Some(state) = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().get_mut(&pos) { state.2 = false; }
    }
}

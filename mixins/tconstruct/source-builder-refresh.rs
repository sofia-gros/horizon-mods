fn refresh_local_builder_choices(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() || crate::game::tconstruct_ui::kind(game) != Some(false) { return; }
    if let Some(pos) = game.container {
        crate::game::tconstruct_source_builder_world::refresh(&mut game.world, pos);
    }
}

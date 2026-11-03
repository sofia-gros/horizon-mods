fn tconstruct_machine_remote_screen(game: &mut crate::game::Game<'_>, target: Option<(i32, i32, i32)>) -> std::ops::ControlFlow<()> {
    if game.remote.is_some() {
        *game.world.chunks.__horizon_mixins_tconstruct_remote_screen_position_mut() = target;
    }
    std::ops::ControlFlow::Continue(())
}

fn tconstruct_glow_process(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<Result<()>> {
    if let Err(error) = crate::game::tconstruct_glowing_world::process_support_changes(game) {
        return std::ops::ControlFlow::Break(Err(error));
    }
    std::ops::ControlFlow::Continue(())
}

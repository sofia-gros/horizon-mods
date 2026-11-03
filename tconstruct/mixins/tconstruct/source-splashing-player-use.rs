fn tconstruct_splashing_player_use(game: &mut crate::game::Game<'_>, target: Option<crate::game::InteractTarget>) -> std::ops::ControlFlow<Result<bool>> {
    if target.is_some_and(|target| crate::game::tconstruct_source_splashing_player::queue(game, target.id)) {
        game.swing_arm();
        std::ops::ControlFlow::Break(Ok(true))
    } else {
        std::ops::ControlFlow::Continue(())
    }
}

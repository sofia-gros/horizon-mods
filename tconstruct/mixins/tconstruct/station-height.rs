fn tconstruct_screen_height(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<f32> {
    if crate::game::tconstruct_ui::kind(game).is_some() {
        std::ops::ControlFlow::Break(184.0)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}

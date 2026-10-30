fn tconstruct_melter_height(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<f32> {
    if crate::game::tconstruct_ui::is_melter(game) {
        std::ops::ControlFlow::Break(166.0)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}

fn tconstruct_drag_guard(game: &crate::game::Game<'_>, ops: &[(Option<usize>, i32, horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    if crate::game::tconstruct_ui::kind(game).is_some() && ops.iter().any(|(_, _, input)| *input != horizon_client::menu::Input::Pickup) {
        return std::ops::ControlFlow::Break(true);
    }
    std::ops::ControlFlow::Continue(())
}

fn station_name_typing(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<bool> {
    if crate::game::tconstruct_name_ui::focused(game) { std::ops::ControlFlow::Break(true) }
    else { std::ops::ControlFlow::Continue(()) }
}

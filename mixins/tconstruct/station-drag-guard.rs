fn tconstruct_drag_guard(game: &crate::game::Game<'_>, ops: &[(Option<usize>, i32, horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    let builder_quick = crate::game::tconstruct_ui::kind(game) == Some(false) && game.remote.is_some()
        && ops.iter().all(|(slot, _, input)| *input == horizon_client::menu::Input::QuickMove && slot.is_some_and(|slot| slot < 36 || slot == 322));
    if crate::game::tconstruct_ui::kind(game).is_some() && !builder_quick && ops.iter().any(|(_, _, input)| *input != horizon_client::menu::Input::Pickup) {
        return std::ops::ControlFlow::Break(true);
    }
    std::ops::ControlFlow::Continue(())
}

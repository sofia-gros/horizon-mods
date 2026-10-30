fn tconstruct_drag_guard(game: &crate::game::Game<'_>, ops: &[(Option<usize>, i32, horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    // The current Melter uses three live slots in the shared chest protocol.
    // Do not allow quick-move or drag to place items in its hidden slots.
    if crate::game::tconstruct_ui::is_melter(game) && ops.iter().any(|(slot, _, input)| {
        *input != horizon_client::menu::Input::Pickup || slot.is_some_and(|slot| (303..=326).contains(&slot) || (328..=353).contains(&slot))
    }) { return std::ops::ControlFlow::Break(true); }
    let builder_quick = crate::game::tconstruct_ui::kind(game) == Some(false) && game.remote.is_some()
        && ops.iter().all(|(slot, _, input)| *input == horizon_client::menu::Input::QuickMove && slot.is_some_and(|slot| slot < 36 || slot == 322));
    if crate::game::tconstruct_ui::kind(game).is_some() && !builder_quick && ops.iter().any(|(_, _, input)| *input != horizon_client::menu::Input::Pickup) {
        return std::ops::ControlFlow::Break(true);
    }
    std::ops::ControlFlow::Continue(())
}

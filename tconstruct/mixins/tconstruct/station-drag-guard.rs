fn tconstruct_drag_guard(game: &crate::game::Game<'_>, ops: &[(Option<usize>, i32, horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    if crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) {
        let (count, offset) = crate::game::tconstruct_source_smeltery_ui::page(game);
        if ops.iter().any(|(slot, _, input)| {
            if *input != horizon_client::menu::Input::Pickup { return true; }
            let Some(slot) = *slot else { return true; };
            (36..300).contains(&slot) || slot >= 354 || (slot >= 300 && crate::game::tconstruct_source_smeltery_layout::melting_index(count, offset, slot - 300).is_none())
        }) { return std::ops::ControlFlow::Break(true); }
    }
    // The current Melter uses three live slots in the shared chest protocol.
    // Do not allow quick-move or drag to place items in its hidden slots.
    if crate::game::tconstruct_ui::is_melter(game) && ops.iter().any(|(slot, _, input)| {
        *input != horizon_client::menu::Input::Pickup || slot.is_some_and(crate::game::tconstruct_source_melter_layout::hidden)
    }) { return std::ops::ControlFlow::Break(true); }
    if crate::game::tconstruct_ui::is_melter(game) && ops.iter().any(|(slot, _, input)| {
        *slot == Some(327) && *input == horizon_client::menu::Input::Pickup
    }) {
        if let Some(stack) = game.carried.as_ref() {
            if game.res.providers().burn_time(game.res.items(), stack, false) <= 3 {
                return std::ops::ControlFlow::Break(true);
            }
        }
    }
    let builder_quick = crate::game::tconstruct_ui::kind(game) == Some(false) && game.remote.is_some()
        && ops.iter().all(|(slot, _, input)| *input == horizon_client::menu::Input::QuickMove && slot.is_some_and(|slot| slot < 36 || slot == 322));
    if crate::game::tconstruct_ui::kind(game).is_some() && !builder_quick && ops.iter().any(|(_, _, input)| *input != horizon_client::menu::Input::Pickup) {
        return std::ops::ControlFlow::Break(true);
    }
    std::ops::ControlFlow::Continue(())
}

fn tconstruct_crafting_menu(game: &mut crate::game::Game<'_>, ops: &[(Option<usize>, i32, horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    use horizon_client::menu::Input;
    if crate::game::tconstruct_crafting_station::position(game).is_none() { return std::ops::ControlFlow::Continue(()); }
    crate::game::tconstruct_crafting_station::refresh(game);
    for &(slot, _, input) in ops {
        if let (Some(slot), Input::QuickMove | Input::Swap) = (slot, input) { game.unselect_bundle(slot); }
    }
    let ops = ops.to_vec();
    let handled = game.menu_session(move |menu, env, to_menu| {
        let mut clicks = Vec::new();
        for (slot, button, input) in ops {
            let slot = match slot {
                Some(200) if input != Input::QuickMove => return None,
                Some(slot) => to_menu(slot)? as i32,
                None => -999,
            };
            clicks.push((slot, button, input));
        }
        for (slot, button, input) in clicks {
            if menu.click(env, slot, button, input).is_err() { return Some(false); }
        }
        Some(true)
    });
    crate::game::tconstruct_crafting_station::sync(game);
    std::ops::ControlFlow::Break(handled)
}

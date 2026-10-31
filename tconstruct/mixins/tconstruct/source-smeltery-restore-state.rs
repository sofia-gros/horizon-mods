fn smeltery_restore_state(game: &mut crate::game::Game<'_>) {
    if !crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) { return; }
    let Some(pos) = crate::game::tconstruct_source_smeltery_ui::position(game) else { return; };
    let Some((window, state)) = game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_menu_state().get(&pos).copied() else { return; };
    if let Some(remote) = game.remote.as_mut() {
        if remote.menu.as_ref().is_some_and(|menu| menu.0 == window) { remote.state_id = state; }
    }
}

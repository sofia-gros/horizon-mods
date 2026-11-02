fn tconstruct_menu_click_state(game: &mut crate::game::Game<'_>, container: i32) {
    if container == 0 { return; }
    if let Some((window, state)) = *game.world.chunks.__horizon_mixins_tconstruct_menu_window_state() {
        if window == container {
            if let Some(remote) = game.remote.as_mut() { remote.state_id = state; }
        }
    }
}

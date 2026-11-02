fn tconstruct_chest_remote_click(game: &mut crate::game::Game<'_>, ops: &[(Option<usize>, i32, horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    if game.remote.is_none() || crate::game::tconstruct_chest_ui::kind(game).is_none() { return std::ops::ControlFlow::Continue(()); }
    let mut clicks=Vec::with_capacity(ops.len());
    for &(slot,button,input) in ops {
        let index=match slot {
            None => -999,
            Some(s @ 300..=331) => (s-300) as i16,
            Some(s @ 9..=35) => (s-9+54) as i16,
            Some(s @ 0..=8) => (s+81) as i16,
            _ => return std::ops::ControlFlow::Break(false),
        };
        clicks.push((index,button,input,Vec::new(),game.carried.clone()));
    }
    game.send_clicks(game.remote_menu_id(),clicks);
    std::ops::ControlFlow::Break(true)
}

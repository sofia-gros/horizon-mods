fn tconstruct_double_client_click(game: &mut crate::game::Game<'_>, ops: &[(Option<usize>, i32, horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    if crate::game::tconstruct_ui::kind(game) == Some(true) && std::env::var_os("TCONSTRUCT_STATION_TRACE").is_some() {
        let (window, menu) = game.remote_menu();
        for (slot, button, input) in ops {
            eprintln!("station trace remote_ops window={window} slot={slot:?} mapped={:?} button={button} input={}", slot.and_then(|slot| crate::game::Game::menu_slot_index(&menu, slot)), input_ordinal(*input));
        }
    }
    if game.remote_menu_title().map(|title| title.plain()).as_deref() != Some("TConstruct Part Builder Double") { return Continue(()); }
    let mut clicks = Vec::with_capacity(ops.len());
    for &(slot, button, input) in ops {
        let index = match slot {
            None => -999,
            Some(300..=380) => (slot.unwrap() - 300) as i16,
            Some(9..=35) => (slot.unwrap() - 9 + 81) as i16,
            Some(0..=8) => (slot.unwrap() + 108) as i16,
            _ => return Break(false),
        };
        clicks.push((index, button, input, Vec::new(), game.carried.clone()));
    }
    game.send_clicks(game.remote_menu_id(), clicks);
    Break(true)
}

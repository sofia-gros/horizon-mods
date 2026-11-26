fn pluto_menu_switch_guest_flush(game: &mut crate::game::Game<'_>, conn: u32) {
    if let Some(mut host) = game.lan.take() {
        game.apply_queued_menu_switch_for_guest(&mut host, conn);
        if game.lan.is_none() {
            game.lan = Some(host);
        }
    }
}

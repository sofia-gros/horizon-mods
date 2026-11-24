fn pluto_menu_switch_guest_flush(value: (), game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32) {
    game.apply_queued_menu_switch_for_guest(host, conn);
    value
}

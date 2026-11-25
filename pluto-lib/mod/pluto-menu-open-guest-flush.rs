// A LAN guest's queued menu opens (pluto-menu-open), applied at the end of the guest's
// turn through the host's own guest container open: the settle point in the guest's use
// only marked them ready or dropped them, having no hold of the guest's menu.
fn pluto_menu_open_guest_flush(value: (), game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost) {
    game.apply_ready_menu_opens_for_guests(host);
    value
}

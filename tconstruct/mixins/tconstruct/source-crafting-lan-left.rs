fn tconstruct_crafting_lan_left(game: &mut crate::game::Game<'_>, host: &mut LanHost, guest: &mut Guest) {
    crate::game::lan_crafting::tconstruct_crafting_lan::close(game, host, guest.conn, guest.placed_at, None);
}

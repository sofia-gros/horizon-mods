fn tconstruct_crafting_lan_close(game: &mut crate::game::Game<'_>, host: &mut LanHost, conn: u32, window: Option<i32>) {
    let Some(joined) = host.hub.guests.iter().find(|guest| guest.conn == conn).map(|guest| guest.placed_at) else { return; };
    crate::game::lan_crafting::tconstruct_crafting_lan::close(game, host, conn, joined, window);
}

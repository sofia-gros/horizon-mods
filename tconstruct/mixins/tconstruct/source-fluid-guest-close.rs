fn fluid_guest_close(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32) {
    if let Some(index) = host.hub.guests.iter().position(|guest| guest.conn == conn) {
        crate::game::tconstruct_source_fluid_return::live(game, host, index);
    }
}

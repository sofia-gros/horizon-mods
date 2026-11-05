fn tconstruct_maintained_guest_refresh(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32) {
    let _ = game.as_guest(host,conn,|actor|crate::game::tconstruct_maintained_world::refresh(actor));
}

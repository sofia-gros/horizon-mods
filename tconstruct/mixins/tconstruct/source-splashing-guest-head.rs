// Scope the one-player shortcut to the current interact packet only. A guest
// action with another placed player must use the full host-backed path.
fn tconstruct_splashing_guest_head(game: &mut crate::game::Game<'_>, host: &crate::game::lan::LanHost, conn: u32, packet: &horizon_protocol::packets::play::Serverbound) {
    let pending = game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_scope_mut();
    pending.remove(&conn);
    if matches!(packet, horizon_protocol::packets::play::Serverbound::Interact(_)) {
        let alone = host.hub.guests.iter().filter(|guest| guest.placed).all(|guest| guest.conn == conn);
        pending.insert(conn, alone);
    }
}

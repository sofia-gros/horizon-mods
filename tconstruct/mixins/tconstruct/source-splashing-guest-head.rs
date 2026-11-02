// Scope other player positions to this actor's current interaction packet.
// The hook cannot mutate them; their effect application is still unsupported.
fn tconstruct_splashing_guest_head(game: &mut crate::game::Game<'_>, host: &crate::game::lan::LanHost, conn: u32, packet: &horizon_protocol::packets::play::Serverbound) {
    let nearby = if matches!(packet, horizon_protocol::packets::play::Serverbound::Interact(_)) {
        host.hub.guests.iter().find(|guest| guest.conn == conn && guest.placed).map(|actor| {
            let mut positions: Vec<[f64; 3]> = host.hub.guests.iter().filter(|guest| guest.placed && guest.conn != conn && guest.dimension == actor.dimension).map(|guest| guest.position).collect();
            if host.owner.is_none() {
                if let Some(player) = game.player.as_ref() { positions.push([player.position.x, player.position.y, player.position.z]); }
            }
            positions
        })
    } else { None };
    let pending = game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_scope_mut();
    pending.remove(&conn);
    if let Some(positions) = nearby { pending.insert(conn, positions); }
}

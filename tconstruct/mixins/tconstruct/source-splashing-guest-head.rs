// Snapshot guest identity and bounds for this one interaction packet.
fn tconstruct_splashing_guest_head(game: &mut crate::game::Game<'_>, host: &crate::game::lan::LanHost, conn: u32, packet: &horizon_protocol::packets::play::Serverbound) {
    let nearby = if matches!(packet, horizon_protocol::packets::play::Serverbound::Interact(_)) {
        host.hub.guests.iter().find(|guest| guest.conn == conn && guest.placed).map(|actor| {
            let mut players: Vec<(Option<u32>, i32, [f64; 3], [f64; 3], [f64; 3], bool)> = host.hub.guests.iter().filter(|guest| guest.placed && guest.conn != conn && guest.dimension == actor.dimension).map(|guest| {
                let bounds = guest.bounds(); let min = bounds.min_position(); let max = bounds.max_position();
                (Some(guest.conn), guest.entity_id, guest.position, [min.x, min.y, min.z], [max.x, max.y, max.z], guest.motion.is_some())
            }).collect();
            // Local host targeting is not handled by the guest swap path.
            if host.owner.is_none() { if let Some(player) = game.player.as_ref() {
                let pos = [player.position.x, player.position.y, player.position.z];
                players.push((None, 0, pos, pos, pos, false));
            }}
            players
        })
    } else { None };
    game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_pending_mut().remove(&conn);
    let pending = game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_scope_mut();
    pending.remove(&conn);
    if let Some(players) = nearby { pending.insert(conn, players); }
}

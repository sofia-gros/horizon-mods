// START carries the actual hit face. Preserve it for STOP and delayed breaks.
fn tconstruct_area_guest_hit(game: &mut crate::game::Game<'_>, conn: u32, packet: &horizon_protocol::packets::play::Serverbound) {
    if let horizon_protocol::packets::play::Serverbound::PlayerAction(action) = packet {
        match action.action {
            0 => { game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits_mut().insert(conn, (action.pos, direction(i32::from(action.direction)))); }
            2 => { game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits_mut().remove(&conn); }
            _ => {}
        }
    }
}

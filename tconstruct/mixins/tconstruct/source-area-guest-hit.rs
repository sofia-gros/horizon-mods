// START carries the hit face; preserve it for STOP and delayed breaks.
fn tconstruct_area_guest_hit(game: &mut crate::game::Game<'_>, host: &crate::game::lan::LanHost, conn: u32, packet: &horizon_protocol::packets::play::Serverbound) {
    if let horizon_protocol::packets::play::Serverbound::PlayerAction(action) = packet {
        match action.action {
            0 => {
                let face = direction(i32::from(action.direction));
                if let Some(protected) = self::tconstruct_source_area_protection::protected(game, host, conn, action.pos, face) {
                    game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits_mut().insert(conn, (action.pos, face, protected));
                }
            }
            2 => { game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits_mut().remove(&conn); }
            _ => {}
        }
    }
}

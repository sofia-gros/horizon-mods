// Capture the server's existing spawn-protection decision for the whole source
// area before the guest is temporarily installed as the acting player.
fn tconstruct_transform_guest_hit(game: &mut crate::game::Game<'_>, host: &crate::game::lan::LanHost, conn:u32, packet:&horizon_protocol::packets::play::Serverbound) {
    if let horizon_protocol::packets::play::Serverbound::UseItemOn(action)=packet {
        let face=direction(action.face);
        if let Some(blocked)=self::tconstruct_source_area_protection::protected(game,host,conn,action.pos,face) {
            game.world.chunks.__horizon_mixins_tconstruct_transform_guest_hits_mut().insert(conn,(action.pos,blocked));
        } else {
            game.world.chunks.__horizon_mixins_tconstruct_transform_guest_hits_mut().remove(&conn);
        }
    }
}

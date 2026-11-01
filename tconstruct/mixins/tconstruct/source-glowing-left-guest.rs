// Intercept only an eligible source left-click, retaining the host's protection checks.
fn tconstruct_glow_left_guest(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, packet: &horizon_protocol::packets::play::Serverbound) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let S::PlayerAction(action) = packet else { return Continue(()); };
    if action.action != 0 { return Continue(()); }
    let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else { return Continue(()); };
    let guest = &host.hub.guests[gi];
    if guest.dead || guest.spectator() { return Continue(()); }
    let Some(stack) = guest.inventory.get(guest.selected).and_then(Option::as_ref).cloned() else { return Continue(()); };
    if !crate::game::tconstruct_glowing_world::eligible(game,&stack,true) { return Continue(()); }
    if game.under_spawn_protection(host,gi,action.pos) { return Continue(()); }
    let face = direction(i32::from(action.direction));
    let Some(x)=action.pos[0].checked_add(face[0]) else { return Continue(()); };
    let Some(y)=action.pos[1].checked_add(face[1]) else { return Continue(()); };
    let Some(z)=action.pos[2].checked_add(face[2]) else { return Continue(()); };
    let target=[x,y,z];
    if game.under_spawn_protection(host,gi,target) {
        host.hub.ack(conn,action.sequence);
        for pos in [action.pos,target] {
            let state=game.world.block(pos[0],pos[1],pos[2]).0 as i32;
            host.hub.send_to(conn,&C::BlockUpdate(play::BlockUpdate { pos,state }));
        }
        return Break(());
    }
    let outcome=game.as_guest(host,conn,|g| crate::game::tconstruct_glowing_world::activate_for(g,&stack,action.pos,face,true,true));
    let Some(Some(result))=outcome else { return Continue(()); };
    if let Err(error)=result { eprintln!("tconstruct Glowing left-click: {error}"); }
    host.hub.ack(conn,action.sequence);
    for pos in [action.pos,target] {
        let state=game.world.block(pos[0],pos[1],pos[2]).0 as i32;
        host.hub.send_to(conn,&C::BlockUpdate(play::BlockUpdate { pos,state }));
    }
    Break(())
}

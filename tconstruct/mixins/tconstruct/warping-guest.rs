fn tconstruct_warping_guest(
    game: &mut crate::game::Game<'_>,
    host: &mut LanHost,
    swap: &GuestSwap,
) {
    let Some((at, rotation)) = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_warping_teleport_mut()
        .remove(&swap.conn)
    else {
        return;
    };
    let Some(p) = game.player.as_ref() else {
        return;
    };
    let Some(guest) = host.hub.guests.get(swap.i) else {
        return;
    };
    if game.lan_acting != Some(swap.conn)
        || guest.conn != swap.conn
        || guest.vehicle.is_some()
        || p.teleported
        || p.is_passenger
        || [p.position.x, p.position.y, p.position.z] != at
        || [game.yaw, game.pitch] != rotation
        || [guest.yaw, guest.pitch] != rotation
    {
        return;
    }
    host.hub.teleport_keeping_motion(swap.i, at);
}

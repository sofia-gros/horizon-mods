fn tconstruct_dreambound_jump_server(
    game: &mut Game<'_>,
    host: &mut LanHost,
    conn: u32,
    packet: &S,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let S::CustomPayload(message) = packet else {
        return Continue(());
    };
    if message.channel != "tconstruct:dreambound_jump" {
        return Continue(());
    }
    if message.data.as_slice() != [1] || host.hub.recording() {
        return Break(());
    }
    let Some(guest) = host
        .hub
        .guests
        .iter()
        .find(|guest| guest.conn == conn && host.hub.here(guest))
    else {
        return Break(());
    };
    if guest.dead
        || guest.spectator()
        || guest.sleeping
        || guest.vehicle.is_some()
        || !guest.client_loaded()
        || guest.awaiting_teleport.is_some()
    {
        return Break(());
    }
    let _ = game.as_guest(host, conn, |game| {
        let capacity =
            crate::game::tconstruct_dreambound_jump_levels::capacity(&game.world, &game.armor)
                .unwrap_or(0);
        let Some(player) = game.player.as_mut() else {
            return;
        };
        let mut motion = horizon_client::motion::MotionWorld {
            entities: &[],
            player,
            scene: &mut game.world,
            assets: &game.pack.assets,
            collision_binding: game.collision,
            resources: game.motion,
        };
        if let Err(error) = horizon_client::motion::tconstruct_dreambound_jump_motion::attempt(
            &mut motion,
            capacity,
        ) {
            game.error = Some(error);
        }
    });
    Break(())
}

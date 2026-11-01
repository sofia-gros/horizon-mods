// Resolve each acting guest with the host's own spawn-protection predicate.
fn tconstruct_glow_guest_policy(game: &mut crate::game::Game<'_>, host: &crate::game::lan::LanHost, conn: u32) {
    let Some(gi) = host.hub.guests.iter().position(|guest| guest.conn == conn) else {
        game.world.chunks.__horizon_mixins_tconstruct_glow_guest_protection_mut().remove(&conn);
        return;
    };
    let restriction = game.level.world_spawn.and_then(|spawn| {
        game.under_spawn_protection(host, gi, spawn)
            .then_some((spawn, host.hub.spawn_protection as u32))
    });
    game.world.chunks.__horizon_mixins_tconstruct_glow_guest_protection_mut().insert(conn, restriction);
}

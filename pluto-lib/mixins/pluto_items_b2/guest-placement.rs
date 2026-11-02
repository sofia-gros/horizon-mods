// Resolve the acting guest through the host's own spawn-protection predicate.
// Derived from Tinkers' source-glowing-guest-policy.rs at repository commit
// ab686d593cbf61564b54e9ed7b2b8b1ae280214c (MIT).
fn pluto_guest_placement_policy(
    game: &mut crate::game::Game<'_>,
    host: &crate::game::lan::LanHost,
    conn: u32,
) {
    let Some(index) = host.hub.guests.iter().position(|guest| guest.conn == conn) else {
        game.world
            .chunks
            .__horizon_mixins_pluto_items_b2_placement_guest_check_mut()
            .remove(&conn);
        return;
    };
    let restriction = game.level.world_spawn.and_then(|spawn| {
        game.under_spawn_protection(host, index, spawn)
            .then_some((spawn, host.hub.spawn_protection as u32))
    });
    game.world
        .chunks
        .__horizon_mixins_pluto_items_b2_placement_guest_check_mut()
        .insert(conn, restriction);
}

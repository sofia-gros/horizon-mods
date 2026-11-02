fn tconstruct_necrotic_melee_pvp_head(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, victim: Victim, strike: &Strike) {
    let contexts = game.world.chunks.__horizon_mixins_tconstruct_necrotic_melee_pvp_mut();
    for parent in contexts.iter_mut() { *parent = None; }
    let Victim::Guest(victim_conn) = victim;
    let victim_data = host.hub.guest(victim_conn).and_then(|g| g.motion.as_deref().map(|p| (horizon_client::lan::playerdata::uuid_ints(g.uuid), p.health.health())));
    let actor = match strike.attacker {
        Some(conn) if conn != victim_conn => game.as_guest(host, conn, |g| { let uuid = g.acting_player_uuid(); Some((uuid, g.held()?.clone())) }).flatten(),
        Some(_) => None,
        None => { let uuid = game.acting_player_uuid(); game.held().cloned().map(|stack| (uuid, stack)) },
    };
    let entry = victim_data.zip(actor).and_then(|((victim_uuid, health), (attacker_uuid, stack))| {
        if !health.is_finite() || !stack.item.starts_with("tconstruct:") || crate::game::tconstruct_source_melee_shape::intact(&stack).is_none() { return None; }
        Some((strike as *const Strike as usize, victim_conn, victim_uuid, health, strike.attacker, attacker_uuid, stack))
    });
    game.world.chunks.__horizon_mixins_tconstruct_necrotic_melee_pvp_mut().push(entry);
}

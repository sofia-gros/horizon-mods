// Snapshot each actor's existing slot modifiers before guest_replay borrows
// its motion mutably. Every invocation replaces the prior actor's snapshot.
fn tconstruct_featherweight_guest_use_head(game: &mut crate::game::Game<'_>, host: &crate::game::lan::LanHost, conn: u32) {
    let bonus = host.hub.guest(conn).filter(|g| g.vehicle.is_none()).and_then(|g| g.motion.as_deref()).map(|motion| {
        let mut bonus = 0.0_f64;
        if let Some((_, _, instance)) = motion.attributes.instances().find(|(_, key, _)| key.name() == "minecraft:movement_speed") {
            for slot in ["head", "chest", "legs", "feet"] {
                let id = format!("tconstruct:featherweight_movement/{slot}");
                if let Some(modifier) = instance.modifier(&id) {
                    let amount = modifier.amount();
                    if amount.is_finite() && amount > 0.0 { bonus += amount; }
                }
            }
        }
        bonus
    });
    *game.world.chunks.__horizon_mixins_tconstruct_featherweight_guest_use_snapshot_mut() = bonus.map(|value| (conn, value));
}

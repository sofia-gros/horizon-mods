fn tconstruct_melee_protection_hit(
    game: &mut crate::game::Game<'_>,
    amount: f32,
    from: Option<[f64;3]>,
    damage_type: &str,
    known: Option<Option<u32>>,
    original: impl FnOnce(&mut crate::game::Game<'_>, f32, Option<[f64;3]>, &str, Option<Option<u32>>) -> bool,
) -> bool {
    // This wrapper is attached only to mob_hits_player, not generic /damage.
    // Do not mistake a projectile's owner for its direct entity, even if a
    // data pack adds that projectile type to the melee protection tag.
    let projectile = game.combat.rocket_hit
        || game.combat.projectile_owner.is_some()
        || game.combat.projectile_attacker.is_some()
        || game.combat.projectile_direct.is_some()
        || game.res.stations().enchantments.damage_tags.get("minecraft:is_projectile")
            .is_some_and(|types|types.contains(damage_type));
    let direct = game.remote.is_none()
        && crate::game::tconstruct_protection_damage::verified_melee_source(damage_type,projectile);
    let saved = game.player.as_mut().map(|player| {
        let context = player.status.__horizon_mixins_tconstruct_melee_protection_source_mut();
        let previous = std::mem::replace(context,direct.then(||damage_type.to_owned()));
        (player.entity_id,previous)
    });
    let result = original(game,amount,from,damage_type,known);
    // The context belongs to one synchronous hit, including canceled hits.
    // Restore nested context without leaking it into /damage or the next tick.
    if let Some((id,previous)) = saved {
        if let Some(player) = game.player.as_mut().filter(|player|player.entity_id == id) {
            *player.status.__horizon_mixins_tconstruct_melee_protection_source_mut() = previous;
        }
    }
    result
}

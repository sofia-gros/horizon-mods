/// Scope the fire-immune predicate to one synchronous hit. Neither a source
/// position nor the native nearest-mob fallback is proof of a causing entity.
pub(super) fn wrap(
    game: &mut super::Game<'_>, amount: f32, from: Option<[f64;3]>, damage_type: &str,
    known: Option<Option<u32>>,
    original: impl FnOnce(&mut super::Game<'_>,f32,Option<[f64;3]>,&str,Option<Option<u32>>) -> bool,
) -> bool {
    let projectile = game.combat.rocket_hit
        || game.res.stations().enchantments.damage_tags.get("minecraft:is_projectile")
            .is_some_and(|types|types.contains(damage_type));
    let causing = super::tconstruct_source_scorch_protection::causing_id(known,projectile,game.combat.projectile_owner);
    let eligible = game.remote.is_none() && causing.is_some_and(|id|
        game.mobs.iter().any(|mob|mob.id == id && mob.kind.flags.fire_immune));
    let saved = game.player.as_mut().map(|player| {
        let previous = std::mem::replace(player.status.__horizon_mixins_tconstruct_scorch_protection_source_mut(),
            eligible.then(||damage_type.to_owned()));
        (player.entity_id,previous)
    });
    let result = original(game,amount,from,damage_type,known);
    if let Some((id,previous)) = saved {
        if let Some(player) = game.player.as_mut().filter(|player|player.entity_id == id) {
            *player.status.__horizon_mixins_tconstruct_scorch_protection_source_mut() = previous;
        }
    }
    result
}

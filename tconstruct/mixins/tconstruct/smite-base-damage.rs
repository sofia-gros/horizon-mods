fn tconstruct_smite_base_damage(
    game: &mut crate::game::Game<'_>, target_kind: &str, living: bool,
    original: impl FnOnce(&mut crate::game::Game<'_>) -> f32,
) -> f32 {
    let base = original(game);
    if !living || !base.is_finite() { return base; }
    let bonus = game.world.chunks.__horizon_mixins_tconstruct_smite_context().last()
        .filter(|(actor, kind, _)| *actor == game.lan_acting && kind == target_kind)
        .and_then(|(_,_,bonus)| *bonus);
    match bonus { Some(bonus) if (base + bonus).is_finite() => base + bonus, _ => base }
}

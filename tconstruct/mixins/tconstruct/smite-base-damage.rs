fn tconstruct_smite_base_damage(
    base: f32, game: &crate::game::Game<'_>, target_kind: &str, living: bool,
) -> f32 {
    if !living || !base.is_finite() { return base; }
    let bonus = game.world.chunks.__horizon_mixins_tconstruct_smite_context().last()
        .filter(|(actor, kind, _)| *actor == game.lan_acting && kind == target_kind)
        .and_then(|(_,_,bonus)| *bonus);
    match bonus { Some(bonus) if (base + bonus).is_finite() => base + bonus, _ => base }
}

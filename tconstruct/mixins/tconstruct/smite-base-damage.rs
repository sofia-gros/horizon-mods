fn tconstruct_smite_base_damage(
    base: f32, game: &crate::game::Game<'_>, target_kind: &str, living: bool,
) -> f32 {
    if !living || !base.is_finite() { return base; }
    let context = game.world.chunks.__horizon_mixins_tconstruct_smite_context().last()
        .filter(|(actor, kind, _, _)| *actor == game.lan_acting && kind == target_kind);
    match context {
        Some((_, _, bonus, factor)) => crate::game::tconstruct_source_conducting::damage(
            base, bonus.unwrap_or(0.0), *factor).unwrap_or(base),
        None => base,
    }
}

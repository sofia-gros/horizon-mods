fn tconstruct_bad_effect_add(
    game: &crate::game::Game<'_>,
    effect: &mut horizon_client::effects::EffectInstance,
) {
    // Only the new instance's top-level duration changes. Existing and hidden
    // effects retain their own already-applied duration, as in source Added.
    effect.duration = crate::game::tconstruct_crystalstrike_world::effect_duration(
        game,
        effect.id,
        effect.duration,
    );
}

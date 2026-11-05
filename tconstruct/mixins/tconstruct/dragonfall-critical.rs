fn tconstruct_dragonfall_critical(
    game: &crate::game::Game<'_>,
    critical: &mut bool,
    multiplier: &mut f32,
) {
    // Run before native third-party callbacks so they can still deny the crit.
    crate::game::tconstruct_dragonfall_world::critical(game, critical, multiplier);
}

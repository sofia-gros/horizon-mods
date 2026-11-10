fn tconstruct_shield_wear(
    game: &mut crate::game::Game<'_>,
    outcome: horizon_client::health::BlockOutcome,
) {
    crate::game::tconstruct_shield_world::wear(game, outcome);
}

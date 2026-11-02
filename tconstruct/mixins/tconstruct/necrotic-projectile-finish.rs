fn tconstruct_necrotic_projectile_finish(
    game: &mut crate::game::Game<'_>,
    p: &horizon_client::projectiles::Projectile,
    outcome: horizon_client::arrows::EntityHit,
) {
    crate::game::tconstruct_source_necrotic::projectile_finish(game, p, outcome);
}

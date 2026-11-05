fn tconstruct_enderference_arrow_impact(
    game: &mut crate::game::Game<'_>,
    projectile: &horizon_client::projectiles::Projectile,
    hit: horizon_client::projectiles::Hit,
) {
    crate::game::tconstruct_enderference_arrow_world::impact(game, projectile, hit);
}

fn tconstruct_necrotic_projectile_begin(
    game: &mut crate::game::Game<'_>,
    p: &horizon_client::projectiles::Projectile,
    hit: horizon_client::projectiles::Hit,
) {
    crate::game::tconstruct_source_necrotic::projectile_begin(game, p, hit);
}

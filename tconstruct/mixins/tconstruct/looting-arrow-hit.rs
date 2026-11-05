fn tconstruct_looting_arrow_hit(
    game: &mut crate::game::Game<'_>,
    p: &horizon_client::projectiles::Projectile,
    who: horizon_client::mobs::Who,
) {
    crate::game::tconstruct_looting_arrow_world::hit(game, p, who);
}

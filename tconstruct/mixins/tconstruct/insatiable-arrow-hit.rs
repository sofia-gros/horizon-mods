fn tconstruct_insatiable_arrow_hit(
    game: &mut crate::game::Game<'_>,
    p: &horizon_client::projectiles::Projectile,
    who: horizon_client::mobs::Who,
) {
    crate::game::tconstruct_insatiable_ranged_world::hit(game, p, who);
}

fn tconstruct_necrotic_pvp_end(value: horizon_client::Result<()>, game: &mut crate::game::Game<'_>, p: &horizon_client::projectiles::Projectile) -> horizon_client::Result<()> {
    crate::game::tconstruct_source_necrotic::pvp_end(game, p, value.is_ok());
    value
}

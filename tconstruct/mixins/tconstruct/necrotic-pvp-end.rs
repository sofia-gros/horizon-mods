fn tconstruct_necrotic_pvp_end(value: Result<()>, game: &mut crate::game::Game<'_>, p: &horizon_client::projectiles::Projectile) -> Result<()> {
    crate::game::tconstruct_source_necrotic::pvp_end(game, p, value.is_ok());
    value
}

fn pluto_projectile_impact_head(game: &mut crate::game::Game<'_>, projectile: u32) {
    crate::game::creatures::pluto_projectile_impact_game::on_impact(game, projectile);
}

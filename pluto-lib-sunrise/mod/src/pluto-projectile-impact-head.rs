fn pluto_projectile_impact_head(game: &mut crate::game::Game<'_>, call: mod_api::__private::ProjectileImpactCall<'_>) {
    crate::game::creatures::pluto_projectile_impact_game::on_impact(game, call.projectile);
}

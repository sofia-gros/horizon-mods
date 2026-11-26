fn pluto_projectile_spawn_head(game: &mut crate::game::Game<'_>, projectile: &mut sunrise_client_lib::projectiles::Projectile) {
    crate::game::creatures::pluto_projectile_flight_game::on_spawn(game, projectile);
}

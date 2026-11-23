// HEAD on Game::tick_fangs: the flight pass for the projectiles
// tick_projectiles just flew. The creature tick calls tick_projectiles
// and then tick_fangs back to back, and fangs never read the projectile
// list, so delivering at this head is the tick's return point. A RETURN
// on tick_projectiles itself is not weaveable (its body uses try/?,
// E_UNSUPPORTED_CONTROL_FLOW); on its error path the caller aborts the
// tick with ?, so this head is skipped exactly as a return would be.
fn pluto_projectile_flight_return(game: &mut crate::game::Game<'_>) {
    crate::game::creatures::pluto_projectile_flight_game::after_tick(game);
}

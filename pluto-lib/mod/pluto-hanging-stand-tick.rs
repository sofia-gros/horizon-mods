// HEAD on Game::tick_armor_stands: fancy stands are fire immune at the
// class level, so their fire ticks are cleared before the host tick's
// environment step can light them. The after-pass (fire ticks cleared
// again, clear-stand invisibility) lives in
// pluto-hanging-stand-tick-after.rs.
fn pluto_hanging_stand_tick_head(game: &mut Game<'_>) {
    crate::game::pluto_hanging_game::tick_fancy_stands(game, false);
}

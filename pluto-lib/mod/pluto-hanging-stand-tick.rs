// HEAD on Game::tick_armor_stands: fancy stands are fire immune at the
// class level (their fire ticks are cleared around the tick) and a
// clear stand's invisibility follows its equipment; the paired RETURN
// reach runs the after-pass.
fn pluto_hanging_stand_tick_head(game: &mut Game<'_>) {
    crate::game::pluto_hanging_game::tick_fancy_stands(game, false);
}

// RETURN on Game::tick_armor_stands: the after-pass for fancy stands
// (fire ticks are cleared around the tick by the paired HEAD reach, and
// a clear stand's invisibility follows its equipment).
fn pluto_hanging_stand_tick_return(value: Result<()>, game: &mut Game<'_>) -> Result<()> {
    crate::game::pluto_hanging_game::tick_fancy_stands(game, true);
    value
}

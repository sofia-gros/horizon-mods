// HEAD on Game::tick_cushions: the after-pass for fancy stands (fire
// ticks the vanilla stand tick set are cleared, and a clear stand's
// invisibility follows its equipment). A RETURN on tick_armor_stands
// itself is not weaveable (its body runs closures and `?`,
// E_UNSUPPORTED_CONTROL_FLOW), so the after-pass rides the head of
// tick_cushions: the game tick calls tick_armor_stands and then
// tick_cushions back to back, which makes this head the stands pass's
// return point in the tick sequence. On the command path that calls
// tick_armor_stands directly, the after-pass lands at the next tick's
// cushions head instead.
fn pluto_hanging_stand_tick_return(game: &mut Game<'_>) {
    crate::game::pluto_hanging_game::tick_fancy_stands(game, true);
}

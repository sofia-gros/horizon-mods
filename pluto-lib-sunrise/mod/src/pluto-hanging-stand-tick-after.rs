// HEAD on Game::tick_cushions: the pinned tick sequence calls it
// immediately after tick_armor_stands, and nothing between the two
// reads stand state, so the fancy stands' after-pass runs here (fire
// ticks cleared again so a stand never renders as burning, and a clear
// stand's invisibility following its equipment). A RETURN on
// tick_armor_stands cannot be woven (E_UNSUPPORTED_CONTROL_FLOW: the
// host body calls beneath references and through closures).
fn pluto_hanging_stand_tick_after(game: &mut Game<'_>) {
    crate::game::pluto_hanging_game::tick_fancy_stands(game, true);
}

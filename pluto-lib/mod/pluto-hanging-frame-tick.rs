// RETURN on Game::tick_item_frames: the fancy frames' rotation timers,
// clear-frame visibility and the comparator registry refresh run after the
// host's own survival checks.
fn pluto_hanging_frame_tick(game: &mut Game<'_>) {
    crate::game::pluto_hanging_game::tick_fancy_frames(game);
}

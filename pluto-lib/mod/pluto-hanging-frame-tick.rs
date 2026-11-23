// RETURN on Game::tick_item_frames: the fancy frames' rotation timers,
// clear-frame visibility and the comparator registry refresh run after the
// host's own survival checks.
fn pluto_hanging_frame_tick(value: (), game: &mut Game<'_>) {
    let _ = value;
    crate::game::pluto_hanging_game::tick_fancy_frames(game);
}

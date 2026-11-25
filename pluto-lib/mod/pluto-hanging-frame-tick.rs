// HEAD on Game::tick_item_frames: the fancy frames' rotation timers,
// clear-frame visibility and the comparator registry refresh run at the
// head of the host's own frame tick (in the original the variant
// behaviour is part of the frame entity's own tick). A RETURN here
// cannot be woven: the host body clones a frame and calls beneath the
// reference (E_UNSUPPORTED_CONTROL_FLOW). Running first means a frame
// the host discards in this same pass may advance its timer once more;
// its drops re-read the item and position, never the timer.
fn pluto_hanging_frame_tick(game: &mut Game<'_>) {
    crate::game::pluto_hanging_game::tick_fancy_frames(game);
}

fn pluto_player_animate_tick(game: &mut crate::game::Game<'_>) {
    // The arm swings and break broadcasts the mods queued for the ticking player play at their tick: a
    // request made outside a break has no break-completion seam to play it, so the tick is its seam.
    crate::game::pluto_player_animate_flush::flush(game);
}

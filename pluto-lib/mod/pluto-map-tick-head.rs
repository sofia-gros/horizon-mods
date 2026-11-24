// Before the maps tick runs: the stored map in a worn piece swaps into the offhand.
fn pluto_map_tick_head(game: &mut Game<'_>) {
    crate::game::maps::pluto_map_tick::tick_head(game);
}

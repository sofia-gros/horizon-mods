// The compass tick is the first call after the maps tick on both tick paths (the local
// player's equipment-compass pass opens with it, and a guest's turn calls it directly):
// the offhand is restored and a guest gets the update packet here, before the compass
// pass itself reads the offhand.
fn pluto_map_tick_return(game: &mut Game<'_>) {
    crate::game::maps::pluto_map_tick::tick_return(game);
}

// After the maps tick returns: the offhand is restored and a guest gets the update packet.
fn pluto_map_tick_return(value: (), game: &mut Game<'_>) {
    crate::game::maps::pluto_map_tick::tick_return(game, value);
}

// Runs PlutoLib's fishing drain at the head of the game's fishing tick: a queued rod use is acted
// on before the tick moves the hook, and a cast whose hook has ended is forgotten.
fn pluto_fishing_tick_head(game: &mut sunrise_client::game::Game<'_>) {
    pluto_fishing_game::drain(game);
}

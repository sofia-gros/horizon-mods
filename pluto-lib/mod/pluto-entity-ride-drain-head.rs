// Queued passenger requests are applied, and the passenger snapshot is
// rebuilt, at the head of the game's own rider synchronisation: a mount
// applied here is validated and seated by that same synchronisation.
fn pluto_entity_ride_drain(game: &mut Game<'_>) {
    super::pluto_entity_ride_access::drain(game);
}

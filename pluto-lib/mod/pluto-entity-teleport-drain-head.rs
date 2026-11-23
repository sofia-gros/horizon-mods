// Queued player teleports are applied at the head of the game's own rider
// synchronisation: the game's server teleport call runs there, in the
// level's own tick context, for the players this level holds.
fn pluto_entity_teleport_drain(game: &mut Game<'_>) {
    super::pluto_entity_teleport_access::drain(game);
}

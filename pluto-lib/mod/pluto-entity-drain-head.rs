// The entity reach's queued requests are applied at the head of the game's
// own rider synchronisation, in the level's own tick context, in the order
// the transports have always drained: passenger requests first (a mount
// applied here is validated and seated by that same synchronisation), then
// pushes (the game's knockback and motion paths), then player teleports
// (the game's own server teleport call), then mob off-hand stack edits
// (the game's own whole-stack read and write for that slot).
fn pluto_entity_drain(game: &mut Game<'_>) {
    super::pluto_entity_ride_access::drain(game);
    super::pluto_entity_push_access::drain(game);
    super::pluto_entity_teleport_access::drain(game);
    super::pluto_mob_offhand_access::drain(game);
}

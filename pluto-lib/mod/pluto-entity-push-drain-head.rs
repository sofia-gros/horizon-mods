// Queued entity pushes are applied at the head of the game's own rider
// synchronisation: the game's knockback and motion paths run there, in the
// level's own tick context, for the entities this level holds.
fn pluto_entity_push_drain(game: &mut Game<'_>) {
    super::pluto_entity_push_access::drain(game);
}

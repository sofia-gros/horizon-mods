// Runs before enter_remote_dimension changes the native heard-level authority.
// keep_world is the host body's own `same`: a respawn entry whose dimension is
// the heard level's keeps the client world; anything else replaces it.
fn pluto_client_block_entity_level_replace(game: &Game<'_>, entry: crate::game::remote::RemoteDimensionEntry, spawn: &play::SpawnInfo) {
    let keep_world = matches!(
        entry,
        crate::game::remote::RemoteDimensionEntry::Respawn
            | crate::game::remote::RemoteDimensionEntry::DeathRespawn
    ) && game
        .remote
        .as_ref()
        .is_some_and(|remote| remote.heard_level.as_deref() == Some(spawn.dimension.as_str()));
    crate::game::remote::pluto_client_block_entity_level::before_replace(game, keep_world, spawn.dimension.as_str());
}

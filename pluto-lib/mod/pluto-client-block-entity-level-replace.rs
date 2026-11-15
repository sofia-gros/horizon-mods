// Runs before enter_remote_dimension changes the native heard-level authority.
fn pluto_client_block_entity_level_replace(game: &Game<'_>, keep_world: bool, spawn: &play::SpawnInfo) {
    crate::game::remote::pluto_client_block_entity_level::before_replace(game, keep_world, spawn.dimension.as_str());
}

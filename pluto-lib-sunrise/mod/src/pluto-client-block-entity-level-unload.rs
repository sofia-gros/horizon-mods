// Actual client unload only; a post-load notification for the old level is ignored.
fn pluto_client_block_entity_level_unload(game: &Game<'_>) {
    crate::game::remote::pluto_client_block_entity_level::before_unload(game);
}

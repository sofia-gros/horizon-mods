// Source-only attachment; the attachment-contract guard blocks native use until reviewed.
fn pluto_client_block_entity_tick_head(game: &mut Game<'_>) {
    crate::game::pluto_client_block_entity_tick::tick(game);
}

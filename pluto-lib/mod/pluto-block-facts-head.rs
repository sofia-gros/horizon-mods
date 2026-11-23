fn pluto_block_facts_head(game: &crate::game::Game<'_>) {
    crate::entry::pluto_block_facts::publish_dig_height([game.world.bottom_y(), game.world.top_y()]);
}

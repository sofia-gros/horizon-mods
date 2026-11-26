fn pluto_block_remove_drain_head(game: &mut crate::game::Game<'_>) {
    // Every level operation ends in this flush (Game::with_level), before the flush
    // materialises the operation's drops: removals captured during the operation are
    // drained here, so their drops join the same flush (see pluto-block-remove).
    crate::game::pluto_block_remove::drain(game);
}

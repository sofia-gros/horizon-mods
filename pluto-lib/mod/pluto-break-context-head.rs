fn pluto_break_context_head(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    // The block entity is read before the break's own dispatch runs, while the block is still there: the
    // bridge takes it when it builds the break context, and the dispatch that follows is the same break.
    crate::game::pluto_break_context::snapshot(game, pos);
}

fn tconstruct_spilling_return(value: (), game: &mut crate::game::Game<'_>) -> () {
    crate::game::tconstruct_source_spilling::end(game);
    value
}

fn tconstruct_necrotic_return(value: (), game: &mut crate::game::Game<'_>) -> () {
    crate::game::tconstruct_source_necrotic::end(game);
    value
}

fn tconstruct_bonking_leave(value: (), game: &crate::game::Game<'_>) -> () {
    crate::game::tconstruct_bonking_world::attack_leave(game);
    value
}

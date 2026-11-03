fn tconstruct_chute_hopper_suck(game: &mut crate::game::Game<'_>, pos: (i32, i32, i32), hopper: &View) -> std::ops::ControlFlow<bool> {
    tconstruct_chute_hopper::suck(game, pos, hopper)
}

fn tconstruct_chute_hopper_eject(game: &mut crate::game::Game<'_>, pos: (i32, i32, i32), state: BlockStateId, hopper: &View) -> std::ops::ControlFlow<bool> {
    tconstruct_chute_hopper::eject(game, pos, state, hopper)
}

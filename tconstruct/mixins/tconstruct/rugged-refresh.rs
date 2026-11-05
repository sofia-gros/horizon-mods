fn tconstruct_rugged_refresh(value: (), game: &mut crate::game::Game<'_>) {
    let _ = value;
    crate::game::tconstruct_rugged_world::refresh(game);
}

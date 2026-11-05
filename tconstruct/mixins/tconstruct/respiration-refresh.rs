fn tconstruct_respiration_refresh(value: (), game: &mut crate::game::Game<'_>) {
    let _ = value;
    crate::game::tconstruct_respiration_world::refresh(game);
    crate::game::tconstruct_turtles_grace_world::refresh(game);
}

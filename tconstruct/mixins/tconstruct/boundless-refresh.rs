fn tconstruct_boundless_refresh(value: (), game: &mut crate::game::Game<'_>) {
    let _ = value;
    crate::game::tconstruct_boundless_world::refresh(game);
}

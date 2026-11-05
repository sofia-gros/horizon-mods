fn tconstruct_knockback_refresh(value: (), game: &mut crate::game::Game<'_>) {
    crate::game::tconstruct_knockback_world::refresh(game);
    value
}

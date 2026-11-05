fn tconstruct_projectile_protection_refresh(value: (), game: &mut crate::game::Game<'_>) {
    let _ = value;
    crate::game::tconstruct_projectile_protection_world::refresh(game);
}

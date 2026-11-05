fn tconstruct_knockback_extra(
    game: &mut crate::game::Game<'_>, kind: &str, scale: f32,
    original: impl FnOnce(&mut crate::game::Game<'_>, &str, f32) -> (f32, bool),
) -> (f32, bool) {
    crate::game::tconstruct_knockback_world::refresh(game);
    let value = original(game, kind, scale);
    if kind.trim_start_matches("minecraft:") == "sulfur_cube" { return value; }
    crate::game::tconstruct_knockback_world::extra(game, value)
}

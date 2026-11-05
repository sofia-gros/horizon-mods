fn tconstruct_padded_extra(
    game: &mut crate::game::Game<'_>,
    kind: &str,
    scale: f32,
    original: impl FnOnce(&mut crate::game::Game<'_>, &str, f32) -> (f32, bool),
) -> (f32, bool) {
    let value = original(game, kind, scale);
    crate::game::tconstruct_padded_world::extra(game, value)
}

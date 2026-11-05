fn tconstruct_pierce_before_hit(
    game: &mut crate::game::Game<'_>, kind: &str, scale: f32, index: usize,
    original: impl FnOnce(&mut crate::game::Game<'_>, &str, f32) -> (f32, bool),
) -> (f32, bool) {
    // Source beforeMeleeHit precedes damage, even if the later hit is rejected.
    let result = original(game, kind, scale);
    crate::game::tconstruct_pierce_world::before_hit(game, index, scale);
    crate::game::tconstruct_enderference_world::before_hit(game, index, scale);
    result
}

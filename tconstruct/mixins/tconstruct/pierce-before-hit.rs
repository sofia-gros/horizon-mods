fn tconstruct_pierce_before_hit(
    game: &mut crate::game::Game<'_>, index: usize, result: &(f32, f32, bool, bool),
) {
    // Use the actual cooldown from melee_damage before attack_mob resets it.
    // Both source beforeMeleeHit modules run before the later damage attempt.
    crate::game::tconstruct_pierce_world::before_hit(game, index, result.1);
    crate::game::tconstruct_enderference_world::before_hit(game, index, result.1);
}

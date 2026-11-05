fn tconstruct_padded_hurt(
    game: &mut crate::game::Game<'_>,
    index: usize,
    amount: f32,
    source: horizon_client::mobs::Damage,
    original: impl FnOnce(&mut crate::game::Game<'_>, usize, f32, horizon_client::mobs::Damage) -> bool,
) -> bool {
    crate::game::tconstruct_padded_world::before_hurt(game, index, &source);
    let landed = original(game, index, amount, source);
    crate::game::tconstruct_padded_world::after_hurt(game, index, &source, landed);
    landed
}

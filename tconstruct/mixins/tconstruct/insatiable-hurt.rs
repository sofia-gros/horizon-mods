fn tconstruct_insatiable_hurt(
    game: &mut crate::game::Game<'_>,
    index: usize,
    amount: f32,
    source: horizon_client::mobs::Damage,
    original: impl FnOnce(&mut crate::game::Game<'_>, usize, f32, horizon_client::mobs::Damage) -> bool,
) -> bool {
    let landed = original(game, index, amount, source);
    crate::game::tconstruct_insatiable_world::hit(game, index, landed);
    landed
}

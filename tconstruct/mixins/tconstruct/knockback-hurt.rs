fn tconstruct_knockback_hurt(
    game: &mut crate::game::Game<'_>, index: usize, amount: f32,
    source: horizon_client::mobs::Damage,
    original: impl FnOnce(&mut crate::game::Game<'_>, usize, f32, horizon_client::mobs::Damage) -> bool,
) -> bool {
    let active = crate::game::tconstruct_knockback_world::level(game).is_some()
        && game.mobs.get(index).is_some_and(|mob| mob.kind.id.trim_start_matches("minecraft:") != "sulfur_cube");
    let landed = original(game, index, amount, source);
    if active && landed {
        if let Some(player) = game.player.as_mut() {
            player.velocity.x *= 0.6;
            player.velocity.z *= 0.6;
        }
    }
    landed
}

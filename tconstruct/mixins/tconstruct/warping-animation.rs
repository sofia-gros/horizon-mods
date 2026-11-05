fn tconstruct_warping_animation(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    animation: &mut horizon_world::consumable::UseAnimation,
) {
    if crate::game::tconstruct_warping_world::hinted(stack)
        && (game.remote.is_some()
            || crate::game::tconstruct_warping_world::config(game, stack).is_some())
    {
        *animation = horizon_world::consumable::UseAnimation::Bow;
    }
}

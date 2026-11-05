fn tconstruct_warping_duration(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    duration: &mut i32,
) {
    if crate::game::tconstruct_warping_world::hinted(stack)
        && (game.remote.is_some()
            || crate::game::tconstruct_warping_world::config(game, stack).is_some())
    {
        *duration = 72000;
    }
}

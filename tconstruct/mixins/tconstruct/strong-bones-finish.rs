fn tconstruct_strong_bones_finish(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_items::items::ItemStack,
) {
    crate::game::tconstruct_strong_bones_world::finish(game, stack);
}

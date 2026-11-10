fn tconstruct_worldbound_expire(
    game: &crate::game::Game<'_>,
    stack: &horizon_items::items::ItemStack,
    extra_life: &mut i32,
) {
    crate::game::tconstruct_worldbound_world::expire(game, stack, extra_life);
}

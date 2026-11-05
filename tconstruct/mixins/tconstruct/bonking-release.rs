fn tconstruct_bonking_release(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    entity: Option<i32>,
    time_left: i32,
    used: &mut bool,
) {
    if entity.is_none() && crate::game::tconstruct_bonking_world::release(game, stack, time_left) {
        *used = true;
    }
}

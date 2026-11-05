fn tconstruct_bonking_start(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    entity: Option<i32>,
    duration: &mut i32,
    canceled: &mut bool,
) {
    if entity.is_none() && *duration == 72000 && !*canceled {
        crate::game::tconstruct_bonking_world::start(game, stack);
    }
}

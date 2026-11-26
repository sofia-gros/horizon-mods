fn pluto_world_fluid_use(
    game: &mut crate::game::Game<'_>,
    stack: &sunrise_items::items::ItemStack,
) -> core::ops::ControlFlow<Result<sunrise_client::first_person::UseResult>> {
    crate::game::pluto_world_fluid::use_bucket(game, stack)
}

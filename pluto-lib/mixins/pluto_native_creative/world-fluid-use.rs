fn pluto_world_fluid_use(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    crate::game::pluto_world_fluid::use_bucket(game, stack)
}

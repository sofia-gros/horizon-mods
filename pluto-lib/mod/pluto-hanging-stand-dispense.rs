// HEAD on Game::dispense_item: a registered stand variant item dispenses
// a fancy stand (DispenseFancyArmorStand); anything else runs the host's
// dispense dispatch. The head runs before the host's mod dispense hook,
// so on a served game a fancy stand item is PlutoLib's to dispense.
fn pluto_hanging_stand_dispense(
    game: &mut Game<'_>,
    pos: (i32, i32, i32),
    facing: Direction,
    stack: &ItemStack,
) -> std::ops::ControlFlow<Option<Result<(Option<ItemStack>, i32)>>> {
    let Some(facts) = horizon_client::entry::pluto_hanging_access::stand_for_item(&stack.item) else {
        return std::ops::ControlFlow::Continue(());
    };
    std::ops::ControlFlow::Break(Some(crate::game::pluto_hanging_game::dispense_fancy_stand(
        game, pos, facing, stack, &facts,
    )))
}

// HEAD on Game::dispense_item: a registered stand variant item dispenses
// a fancy stand (DispenseFancyArmorStand); anything else runs the host's
// dispense dispatch. The head runs before the host's mod dispense hook,
// so on a served game a fancy stand item is PlutoLib's to dispense.
fn pluto_hanging_stand_dispense(
    game: &mut Game<'_>,
    pos: (i32, i32, i32),
    facing: Direction,
    stack: &ItemStack,
) -> core::ops::ControlFlow<Result<ControlFlow<(), Option<ItemStack>>>> {
    let Some(facts) = crate::pluto_hanging_access::stand_for_item(&stack.item) else {
        return std::ops::ControlFlow::Continue(());
    };
    match crate::game::pluto_hanging_game::dispense_fancy_stand(game, pos, facing, stack, &facts) {
        Ok((left, event)) => {
            // The host body answers a mod dispense the same way: playSound,
            // then playAnimation; a NOOP behaviour (event 0) plays neither.
            if event != 0 {
                game.dispenser_event(event, pos, 0);
                game.dispenser_event(2000, pos, data_value(facing));
            }
            std::ops::ControlFlow::Break(Ok(std::ops::ControlFlow::Continue(left)))
        }
        Err(error) => std::ops::ControlFlow::Break(Err(error)),
    }
}

fn pluto_use(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) -> std::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    use std::ops::ControlFlow::{Break, Continue};
    use horizon_client::first_person::UseResult;
    use pluto_item_api::UseOutcome;
    if matches!(game.guest_use, Some(crate::game::GuestUse::OnBlock(_))) || !pluto_item_api::contains(&stack.item) { return Continue(()); }
    let Ok(mut data) = crate::game::pluto_item_adapter::read(stack) else { return Break(Ok(UseResult::Fail)); };
    match pluto_item_api::dispatch(&stack.item, &mut data) {
        UseOutcome::Pass => Continue(()),
        UseOutcome::Cancel => Break(Ok(UseResult::Fail)),
        UseOutcome::Handled => {
            let Some(current) = game.held_mut().as_mut().filter(|s| s.item == stack.item) else { return Break(Ok(UseResult::Fail)); };
            if crate::game::pluto_item_adapter::write(current, &data).is_err() { return Break(Ok(UseResult::Fail)); }
            Break(Ok(UseResult::Success { swing: true }))
        }
    }
}

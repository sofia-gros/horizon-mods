fn tconstruct_slurping_use(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) -> std::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    use std::ops::ControlFlow::{Break, Continue};
    if matches!(game.guest_use, Some(crate::game::GuestUse::OnBlock(_))) || crate::game::tconstruct_source_slurping::plan(game, stack).is_none() { return Continue(()); }
    if game.using.is_none() { game.start_using_item(); }
    Break(Ok(horizon_client::first_person::UseResult::Success { swing: false }))
}

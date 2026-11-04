fn tconstruct_bow_use(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) -> std::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    use horizon_client::first_person::UseResult;
    use std::ops::ControlFlow::{Break, Continue};
    if &*stack.item != "tconstruct:longbow" || matches!(game.guest_use, Some(crate::game::GuestUse::OnBlock(_))) { return Continue(()); }
    if crate::game::tconstruct_source_ranged_wear::is_broken(stack) { return Break(Ok(UseResult::Fail)); }
    if game.using.is_none() && (game.creative || game.projectile_slot("minecraft:bow").is_some()) {
        crate::game::tconstruct_source_momentum_ranged_use::start(game);
    }
    Break(Ok(UseResult::Success { swing: false }))
}

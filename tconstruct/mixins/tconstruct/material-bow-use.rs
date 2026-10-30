fn tconstruct_bow_use(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) -> std::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    use horizon_client::first_person::UseResult;
    use std::ops::ControlFlow::{Break, Continue};
    if &*stack.item != "tconstruct:longbow" || matches!(game.guest_use, Some(crate::game::GuestUse::OnBlock(_))) { return Continue(()); }
    if game.using.is_none() && (game.creative || game.projectile_slot("minecraft:bow").is_some()) {
        game.using = Some((game.held_slot(), 0));
    }
    Break(Ok(UseResult::Success { swing: false }))
}

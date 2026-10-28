fn tconstruct_nonfuel_inframe(
    game: &crate::game::Game<'_>,
    slot: usize,
    stack: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    // Integrated-client furnace clicks bypass the server's Fuel slot role.
    // Apply that role to this mod's items only; preserve unrelated host behavior.
    if slot == 301
        && stack.item.starts_with("tconstruct:")
        && matches!(game.container_kind(), Some(horizon_client::containers::BlockEntity::Furnace(_)))
        && game.res.items().get(&stack.item).is_none_or(|item| item.fuel.is_none())
    {
        return Break(true);
    }
    Continue(())
}

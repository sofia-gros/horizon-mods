fn tconstruct_nonfuel_inframe(
    game: &crate::game::Game<'_>,
    slot: usize,
    stack: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    if (327..354).contains(&slot) && !horizon_client::menu::fits_inside_container_items(stack)
        && game.container.and_then(|key| game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&key).copied())
            .is_some_and(|side| game.res.facts().get(game.world.block(side.0, side.1, side.2)).name.ends_with("shulker_box"))
    { return Break(true); }
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

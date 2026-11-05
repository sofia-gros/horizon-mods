fn tconstruct_armor_tanned_wear(
    game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, amount: u32,
    original: impl FnOnce(&mut crate::game::Game<'_>, &horizon_client::items::ItemStack, u32) -> u32,
) -> u32 {
    let amount = crate::game::tconstruct_armor_wear_world::pre_native_loss(game, stack, amount).unwrap_or(amount);
    original(game, stack, amount)
}

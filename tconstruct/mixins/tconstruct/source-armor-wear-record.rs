fn tconstruct_armor_wear_record(
    loss: u32, game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack,
) -> u32 {
    crate::game::tconstruct_armor_wear_world::record(game, stack, loss);
    loss
}

fn tconstruct_offhand_use(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    if !*claimed && crate::game::tconstruct_offhand_attack_world::use_tool(game, stack, None) {
        *claimed = true;
    }
}

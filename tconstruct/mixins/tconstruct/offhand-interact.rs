fn tconstruct_offhand_interact(
    game: &mut crate::game::Game<'_>,
    entity: i32,
    stack: Option<&horizon_client::items::ItemStack>,
    claimed: &mut bool,
) {
    if !*claimed
        && stack.is_some_and(|s| {
            crate::game::tconstruct_offhand_attack_world::use_tool(game, s, Some(entity))
        })
    {
        *claimed = true;
    }
}

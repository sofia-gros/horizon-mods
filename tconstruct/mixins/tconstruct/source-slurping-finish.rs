fn tconstruct_slurping_finish(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, entity: Option<i32>, result: &mut Option<horizon_client::items::ItemStack>) {
    if entity.is_none() { crate::game::tconstruct_source_slurping::finish(game, stack, result); }
}

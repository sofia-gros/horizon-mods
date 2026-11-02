fn tconstruct_slurping_duration(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, duration: &mut i32) {
    if !crate::game::tconstruct_source_slurping::passive_item(stack) && crate::game::tconstruct_source_slurping::plan(game, stack).is_some() { *duration = 21; }
}

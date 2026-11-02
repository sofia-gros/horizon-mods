fn tconstruct_slurping_duration(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, duration: &mut i32) {
    if &*stack.item != "tconstruct:longbow" && crate::game::tconstruct_source_slurping::plan(game, stack).is_some() { *duration = 21; }
}

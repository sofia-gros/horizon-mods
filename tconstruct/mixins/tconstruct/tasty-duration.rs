fn tconstruct_tasty_duration(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, duration: &mut i32) {
    if crate::game::tconstruct_source_tasty::can_start(game, stack) { *duration = 16; }
}

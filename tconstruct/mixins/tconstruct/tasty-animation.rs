fn tconstruct_tasty_animation(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, animation: &mut horizon_world::consumable::UseAnimation) {
    if crate::game::tconstruct_source_tasty::can_start(game, stack) { *animation = horizon_world::consumable::UseAnimation::Eat; }
}

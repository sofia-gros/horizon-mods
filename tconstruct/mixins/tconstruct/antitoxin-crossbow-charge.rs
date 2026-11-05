fn tconstruct_antitoxin_crossbow_charge(value: u32, game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) -> u32 {
    match crate::game::tconstruct_antitoxin_world::draw_bonus(game, stack) {
        Some(bonus) => crate::game::tconstruct_ranged_draw::draw_ticks_with_bonus(stack, bonus),
        None => value,
    }
}

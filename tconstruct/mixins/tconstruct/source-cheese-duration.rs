fn tconstruct_cheese_duration(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, duration: &mut i32) {
    if &*stack.item == "tconstruct:cheese_block" && stack.damage == 0 && stack.extra.is_none() && stack.wire_patch().is_none() && game.items().get(&stack.item).is_some_and(|i| i.consumable.is_none() && i.blocks_attacks.is_none() && i.food.is_none() && i.use_remainder.is_none()) { *duration = 32; }
}

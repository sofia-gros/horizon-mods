fn tconstruct_cheese_start(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, entity: Option<i32>, canceled: &mut bool) {
    if entity.is_some() || &*stack.item != "tconstruct:cheese_block" || stack.damage != 0 || stack.extra.is_some() || stack.wire_patch().is_some() || !game.items().get(&stack.item).is_some_and(|i| i.consumable.is_none() && i.blocks_attacks.is_none() && i.food.is_none() && i.use_remainder.is_none()) { return; }
    if !game.player.as_ref().is_some_and(|p| !p.health.dead_or_dying() && (game.creative || p.health.food.food < 20)) { *canceled = true; }
}

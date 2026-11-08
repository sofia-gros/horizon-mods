fn pluto_item_use_start(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, entity: Option<i32>, canceled: &mut bool) {
    if entity.is_some() || stack.damage != 0 || stack.extra.is_some() || stack.wire_patch().is_some() { return; }
    let Some(spec) = crate::entry::pluto_item_use::get(&stack.item) else { return; };
    // an item the game defines as consumable (or shield, or with a remainder) keeps the game's rules
    if !game.items().get(&stack.item).is_some_and(|i| i.consumable.is_none() && i.blocks_attacks.is_none() && i.food.is_none() && i.use_remainder.is_none()) { return; }
    if spec.food.is_none() || spec.can_always_eat { return; }
    if !game.player.as_ref().is_some_and(|p| !p.health.dead_or_dying() && (game.creative || p.health.food.food < 20)) { *canceled = true; }
}

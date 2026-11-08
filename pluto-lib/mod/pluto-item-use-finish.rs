fn pluto_item_use_finish(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, entity: Option<i32>, result: &mut Option<horizon_client::items::ItemStack>) {
    use crate::game::pluto_item_use_plan::{plan, Remainder};
    if entity.is_some() || game.remote.is_some() { return; }
    let Some(spec) = crate::entry::pluto_item_use::get(&stack.item) else { return; };
    let Some(info) = game.items().get(&stack.item) else { return; };
    if info.consumable.is_some() || info.blocks_attacks.is_some() || info.food.is_some() || info.use_remainder.is_some() { return; }
    let Some(action) = plan(stack, result, game.creative) else { return; };
    if plan(stack, &game.held().cloned(), game.creative) != Some(action) || game.player.is_none() { return; }
    let remainder = match &spec.remainder {
        Some((item, count)) => {
            let Some(max) = game.items().get(item.as_str()).map(|i| i.max_stack) else { return; };
            if max < *count { return; }
            Some((horizon_client::items::ItemStack::new(item.as_str(), *count), max))
        }
        None => None,
    };
    if action == Remainder::Inventory {
        if let Some((remainder, max)) = &remainder {
            // Full inventories refuse consumption rather than relying on a drop that another mod could cancel.
            let mut inventory = game.inventory.clone();
            let mut offhand = game.offhand.clone();
            let mut pending = remainder.clone();
            if crate::game::inventory_add(&mut inventory, &mut offhand, game.selected, *max, &mut pending) { return; }
        }
    }
    // Commit consumption and remainder before any advancement or stat callback can move the hand.
    match action {
        Remainder::Creative => {},
        Remainder::ReplaceHand => *game.held_mut() = remainder.as_ref().map(|(stack, _)| stack.clone()),
        Remainder::Inventory => {
            let Some(held) = game.held_mut().as_mut() else { return; };
            held.count -= 1;
            if let Some((remainder, _)) = &remainder {
                let remaining = game.give(remainder.clone());
                debug_assert!(remaining.is_none());
            }
        }
    }
    if let Some((nutrition, saturation)) = spec.food {
        game.apply_effect_action(horizon_client::effects::EffectAction::Eat(nutrition, saturation));
    }
    game.award_stat("used", &stack.item, 1);
    game.trigger_item("minecraft:consume_item", stack);
    game.post_entity_event("minecraft:eat", horizon_client::mobs::Who::Player);
    // Synchronous callbacks may move the hand; never restore a stale snapshot.
    *result = game.held().cloned();
}

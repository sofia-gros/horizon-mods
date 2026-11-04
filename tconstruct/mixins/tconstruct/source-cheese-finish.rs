fn tconstruct_cheese_finish(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, entity: Option<i32>, result: &mut Option<horizon_client::items::ItemStack>) {
    use crate::game::tconstruct_source_cheese_remainder::{plan, Remainder};
    if entity.is_some() || game.remote.is_some() { return; }
    let Some(info) = game.items().get(&stack.item) else { return; };
    if info.consumable.is_some() || info.blocks_attacks.is_some() || info.food.is_some() || info.use_remainder.is_some() { return; }
    let Some(action) = plan(stack, result, game.creative) else { return; };
    if plan(stack, &game.held().cloned(), game.creative) != Some(action) || game.player.is_none() { return; }
    let Some(max) = game.items().get("tconstruct:cheese_ingot").map(|i| i.max_stack) else { return; };
    if max < 3 { return; }
    let remainder = horizon_client::items::ItemStack::new("tconstruct:cheese_ingot", 3);
    if action == Remainder::Inventory {
        // Full inventories refuse consumption rather than relying on a drop
        // that another mod could cancel. Reuse the host's exact insertion rule.
        let mut inventory = game.inventory.clone();
        let mut offhand = game.offhand.clone();
        let mut pending = remainder.clone();
        if crate::game::inventory_add(&mut inventory, &mut offhand, game.selected, max, &mut pending) { return; }
    }
    // Commit consumption and remainder before any advancement/stat callback
    // can move the hand. Full inventories have already been rejected above.
    match action {
        Remainder::Creative => {},
        Remainder::ReplaceHand => *game.held_mut() = Some(remainder),
        Remainder::Inventory => {
            let Some(held) = game.held_mut().as_mut() else { return; };
            held.count -= 1;
            let remaining = game.give(remainder);
            debug_assert!(remaining.is_none());
        }
    }
    game.apply_effect_action(horizon_client::effects::EffectAction::Eat(3, 0.4));
    game.award_stat("used", &stack.item, 1);
    game.trigger_item("minecraft:consume_item", stack);
    game.post_entity_event("minecraft:eat", horizon_client::mobs::Who::Player);
    // Synchronous callbacks may move the hand; never restore a stale snapshot.
    *result = game.held().cloned();
}

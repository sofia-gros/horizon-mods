fn tconstruct_armor_wear_record(loss: u32, game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) -> u32 {
    use horizon_world::nbt::Tag;
    let dynamic = stack.item.starts_with("tconstruct:")
        && matches!(stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|n| n.get("tic_max_damage")), Some(Tag::Int(n)) if *n > 0);
    // Current registered source armor has no fixed item max. Let the native
    // path own fixed-max items, avoiding a second application of wear.
    let native_fixed = game.res.items().get(&stack.item).is_some_and(|item| item.max_damage > 0);
    if dynamic && !native_fixed {
        if let Some(pending) = game.world.chunks.__horizon_mixins_tconstruct_armor_wear_context_mut().get_mut(&game.lan_acting) {
            pending.push((stack.clone(), loss));
        }
    }
    loss
}

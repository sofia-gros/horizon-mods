fn tconstruct_slurping_bow_tick(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) {
    if &*stack.item != "tconstruct:longbow" || game.remote.is_some() || !matches!(game.using, Some((_, 21))) { return; }
    let mut result = Some(stack.clone());
    crate::game::tconstruct_source_slurping::finish(game, stack, &mut result);
    if let Some(result) = result {
        if game.held().is_some_and(|held| held.damage == stack.damage && held.same_item_same_components(stack)) {
            *game.held_mut() = Some(result);
        }
    }
}

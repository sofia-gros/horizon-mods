fn tconstruct_chest_slot_view(game: &mut crate::game::Game<'_>, slot: usize) -> std::ops::ControlFlow<Option<ItemStack>> {
    if game.remote.is_some() || crate::game::tconstruct_chest_ui::kind(game).is_none() || !(300..556).contains(&slot) { return std::ops::ControlFlow::Continue(()); }
    let stack = game.container.and_then(|key| game.world.block_entities.get(&key)).and_then(|entity| entity.slots().get(slot - 300)).cloned().flatten();
    std::ops::ControlFlow::Break(stack)
}

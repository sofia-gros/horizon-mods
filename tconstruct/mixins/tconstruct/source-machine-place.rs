fn tconstruct_machine_place(game: &mut crate::game::Game<'_>, pos: [i32; 3], state: horizon_world::registry::BlockStateId, stack: &horizon_client::items::ItemStack) {
    if game.remote.is_some() { return; }
    let name = game.res.facts().get(state).name.clone();
    if &*stack.item != &*name { return; }
    crate::game::tconstruct_source_machine_runtime::ensure(game, (pos[0], pos[1], pos[2]), &name);
}

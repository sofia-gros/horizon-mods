fn tconstruct_duct_removal_guard(level: &crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos, state: horizon_world::registry::BlockStateId) -> std::ops::ControlFlow<bool> {
    if !crate::game::tconstruct_source_duct::is_duct(&level.facts.get(state).name) {
        return std::ops::ControlFlow::Continue(());
    }
    let at = (pos.x, pos.y, pos.z);
    // Refuse before loot, item drops, piston writes or command NBT changes.
    // Retrieve the filter with an empty hand first; opaque data stays untouched.
    std::ops::ControlFlow::Break(crate::game::tconstruct_source_machine_runtime::read_duct(level.world, at)
        .is_some_and(|duct| duct.item.is_none()))
}

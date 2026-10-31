fn tconstruct_machine_place(game: &mut crate::game::Game<'_>, pos: [i32; 3], state: horizon_world::registry::BlockStateId, stack: &horizon_client::items::ItemStack) {
    if game.remote.is_some() { return; }
    let name = game.res.facts().get(state).name.clone();
    if &*stack.item != &*name { return; }
    let key = (pos[0], pos[1], pos[2]);
    if !crate::game::tconstruct_source_machine_runtime::ensure(game, key, &name) { return; }
    let Some(capacity) = crate::game::tconstruct_source_smeltery_fuel::capacity(&name) else { return; };
    let Some(saved) = stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("tconstruct:tank")) else { return; };
    let Some(tank) = crate::game::tconstruct_source_melter_state::Tank::from_nbt(saved, capacity) else { return; };
    let Some(kind) = crate::game::tconstruct_source_machine_runtime::kind(&name) else { return; };
    let _ = crate::game::tconstruct_source_melter_store::write(&mut game.world, key, kind, tank.to_nbt());
}

fn tconstruct_machine_open(game: &mut crate::game::Game<'_>, pos: [i32; 3], name: &str) -> std::ops::ControlFlow<bool> {
    if matches!(name, "tconstruct:smeltery_controller" | "tconstruct:foundry_controller") && game.remote.is_none() {
        game.ensure_datapacks();
        let key = (pos[0], pos[1], pos[2]);
        if !crate::game::tconstruct_source_machine_runtime::ensure(game, key, name) { return std::ops::ControlFlow::Break(false); }
        crate::game::tconstruct_source_machine_runtime::refresh_smeltery(game, key);
        if crate::game::tconstruct_source_smeltery_world::detect(game, key).is_none() { return std::ops::ControlFlow::Break(false); }
        let Some(state) = crate::game::tconstruct_source_melter_store::read(&game.world, key, "tconstruct:smeltery")
            .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return std::ops::ControlFlow::Break(false); };
        if state.slots.is_empty() { return std::ops::ControlFlow::Break(false); }
        let Some(horizon_client::containers::BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&key) else { return std::ops::ControlFlow::Break(false); };
        *slots = crate::game::tconstruct_source_smeltery_page::project(&state.slots, 0);
        game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages_mut().insert(key, (state.slots.len(), 0));
        game.container = Some(key);
        return std::ops::ControlFlow::Break(true);
    }
    if !matches!(name, "tconstruct:seared_melter" | "tconstruct:seared_heater") { return std::ops::ControlFlow::Continue(()); }
    if game.remote.is_none() { game.ensure_datapacks(); }
    let key = (pos[0], pos[1], pos[2]);
    if !crate::game::tconstruct_source_machine_runtime::ensure(game, key, name) { return std::ops::ControlFlow::Break(false); }
    game.container = Some(key);
    std::ops::ControlFlow::Break(true)
}

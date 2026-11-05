fn tconstruct_machine_open(game: &mut crate::game::Game<'_>, pos: [i32; 3], name: &str) -> std::ops::ControlFlow<bool> {
    if name == "tconstruct:crafting_station" && game.remote.is_none() {
        return std::ops::ControlFlow::Break(crate::game::tconstruct_crafting_station::open(game, pos));
    }
    let slots = match name { "tconstruct:part_chest" => 256, "tconstruct:tinkers_chest" => 64, _ => 0 };
    if slots != 0 && game.remote.is_none() {
        game.ensure_datapacks();
        let key = (pos[0], pos[1], pos[2]);
        let entity = game.world.block_entities.get_or_insert_with(key, || horizon_client::containers::BlockEntity::Chest(vec![None; slots]));
        if !matches!(entity, horizon_client::containers::BlockEntity::Chest(items) if items.len() == slots) { return std::ops::ControlFlow::Break(false); }
        game.container = Some(key);
        return std::ops::ControlFlow::Break(true);
    }
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
        // Guest requests are awarded only after their actual network menu
        // opens in guest-open.rs, never during a temporary acting context.
        if game.lan_acting.is_none() {
            crate::game::tconstruct_source_container_advancement::opened(game, name);
        }
        return std::ops::ControlFlow::Break(true);
    }
    if !matches!(name, "tconstruct:seared_melter" | "tconstruct:seared_heater") { return std::ops::ControlFlow::Continue(()); }
    if game.remote.is_none() { game.ensure_datapacks(); }
    let key = (pos[0], pos[1], pos[2]);
    if !crate::game::tconstruct_source_machine_runtime::ensure(game, key, name) { return std::ops::ControlFlow::Break(false); }
    game.container = Some(key);
    std::ops::ControlFlow::Break(true)
}

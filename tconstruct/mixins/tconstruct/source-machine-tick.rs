fn tconstruct_machine_tick(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() || game.res.rest.get().is_none() { return; }
    let ticking = game.block_ticking_area();
    let mut positions: Vec<_> = game.world.block_entities.keys().copied().filter_map(|pos| {
        if !ticking(pos.0, pos.2) { return None; }
        let state = game.world.block(pos.0, pos.1, pos.2);
        let name = game.res.facts().get(state).name.clone();
        matches!(&*name, "tconstruct:seared_melter" | "tconstruct:seared_faucet" | "tconstruct:seared_table" | "tconstruct:seared_basin" | "tconstruct:smeltery_controller").then_some((pos, name))
    }).collect();
    positions.sort_by_key(|(pos, _)| *pos);
    for (pos, name) in &positions { if &**name == "tconstruct:smeltery_controller" { crate::game::tconstruct_source_machine_runtime::refresh_smeltery(game, *pos); } }
    for (pos, name) in &positions { if &**name == "tconstruct:seared_melter" { crate::game::tconstruct_source_machine_runtime::tick_melter(game, *pos); } }
    for (pos, name) in &positions { if &**name == "tconstruct:seared_faucet" { crate::game::tconstruct_source_machine_runtime::tick_faucet(game, *pos); } }
    for (pos, name) in &positions { if matches!(&**name, "tconstruct:seared_table" | "tconstruct:seared_basin") { crate::game::tconstruct_source_machine_runtime::tick_casting(game, *pos); } }
}

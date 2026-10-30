fn tconstruct_machine_tick(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() || game.res.rest.get().is_none() { return; }
    let ticking = game.block_ticking_area();
    let positions: Vec<_> = game.world.block_entities.keys().copied().filter(|pos| {
        if !ticking(pos.0, pos.2) { return false; }
        let state = game.world.block(pos.0, pos.1, pos.2);
        &*game.res.facts().get(state).name == "tconstruct:seared_melter"
    }).collect();
    for pos in positions { crate::game::tconstruct_source_machine_runtime::tick_melter(game, pos); }
}

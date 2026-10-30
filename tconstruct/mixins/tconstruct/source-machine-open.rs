fn tconstruct_machine_open(game: &mut crate::game::Game<'_>, pos: [i32; 3], name: &str) -> std::ops::ControlFlow<bool> {
    if !matches!(name, "tconstruct:seared_melter" | "tconstruct:seared_heater") { return std::ops::ControlFlow::Continue(()); }
    if game.remote.is_none() { game.ensure_datapacks(); }
    let key = (pos[0], pos[1], pos[2]);
    if !crate::game::tconstruct_source_machine_runtime::ensure(game, key, name) { return std::ops::ControlFlow::Break(false); }
    game.container = Some(key);
    std::ops::ControlFlow::Break(true)
}

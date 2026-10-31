fn tconstruct_machine_use(game: &mut crate::game::Game<'_>, hit: Option<&Hit>) -> std::ops::ControlFlow<Result<bool>> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() { return Continue(()); }
    let Some(hit) = hit else { return Continue(()); };
    // Match the existing use-item ordering: sneaking with a held item lets
    // that item act instead of opening the machine.
    if game.keys.contains(&KeyCode::ShiftLeft) && (game.stack_in(game.selected).is_some() || game.offhand.is_some()) { return Continue(()); }
    let state = game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
    let name = game.res.facts().get(state).name.clone();
    let pos = (hit.pos[0], hit.pos[1], hit.pos[2]);
    let handled = match &*name {
        "tconstruct:seared_melter" | "tconstruct:seared_heater" => { game.open_menu(hit.pos); true },
        "tconstruct:smeltery_controller" | "tconstruct:foundry_controller" => { game.open_menu(hit.pos); true },
        "tconstruct:seared_faucet" => crate::game::tconstruct_source_machine_runtime::activate_faucet(game, pos, &name),
        "tconstruct:seared_table" | "tconstruct:seared_basin" => crate::game::tconstruct_source_machine_runtime::interact_casting(game, pos, &name),
        "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" | "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge" => crate::game::tconstruct_source_machine_runtime::interact_fuel_tank(game, pos, &name),
        _ => return Continue(()),
    };
    if handled { game.swing_arm(); }
    Break(Ok(handled))
}

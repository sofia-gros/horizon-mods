fn tconstruct_machine_reacts(game: &crate::game::Game<'_>, hit: &super::Hit) -> std::ops::ControlFlow<(bool, DryUse)> {
    use std::ops::ControlFlow::{Break, Continue};
    // Preserve secondary-use behavior, including block placement while sneaking.
    if game.keys.contains(&KeyCode::ShiftLeft) && (game.stack_in(game.selected).is_some() || game.offhand.is_some()) { return Continue(()); }
    let state = game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
    let name = &game.res.facts().get(state).name;
    if matches!(&**name, "tconstruct:seared_melter" | "tconstruct:seared_heater" | "tconstruct:seared_faucet" | "tconstruct:seared_table" | "tconstruct:seared_basin" | "tconstruct:smeltery_controller" | "tconstruct:foundry_controller" | "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" | "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge") {
        return Break((true, DryUse::default()));
    }
    Continue(())
}

fn tconstruct_machine_use(game: &mut crate::game::Game<'_>, hit: Option<&Hit>) -> std::ops::ControlFlow<Result<bool>> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() { return Continue(()); }
    let Some(hit) = hit else { return Continue(()); };
    // Match the existing use-item ordering: sneaking with a held item lets
    // that item act instead of opening the machine.
    if game.keys.contains(&KeyCode::ShiftLeft) && (game.stack_in(game.selected).is_some() || game.offhand.is_some()) { return Continue(()); }
    let state = game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
    let name = game.res.facts().get(state).name.clone();
    if !matches!(&*name, "tconstruct:seared_melter" | "tconstruct:seared_heater") { return Continue(()); }
    game.open_menu(hit.pos);
    game.swing_arm();
    Break(Ok(true))
}

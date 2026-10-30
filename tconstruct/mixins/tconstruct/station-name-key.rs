fn station_name_key(game: &mut crate::game::Game<'_>, key: winit::keyboard::KeyCode) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    if !crate::game::tconstruct_name_ui::focused(game) { return Continue(()); }
    let Some(pos) = game.container else { return Break(false); };
    let (ctrl, shift) = (game.has_control_down(), game.has_shift_down());
    let Some(edit) = game.world.chunks.__horizon_mixins_tconstruct_station_name_edit_mut().get_mut(&pos) else { return Break(false); };
    if !edit.key(key, ctrl, shift) { return Break(false); }
    let name = edit.value.clone();
    crate::game::tconstruct_name_ui::send(game, name);
    Break(true)
}

fn station_name_text(game: &mut crate::game::Game<'_>, text: Option<&str>, backspace: bool) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if !crate::game::tconstruct_name_ui::focused(game) { return Continue(()); }
    let Some(pos) = game.container else { return Break(()); };
    let Some(edit) = game.world.chunks.__horizon_mixins_tconstruct_station_name_edit_mut().get_mut(&pos) else { return Break(()); };
    if backspace { edit.delete_chars(-1); } else if let Some(text) = text { edit.insert_text(text); }
    let name = edit.value.clone();
    crate::game::tconstruct_name_ui::send(game, name);
    Break(())
}

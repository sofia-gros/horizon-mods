fn station_name_shortcut(game: &mut crate::game::Game<'_>, shortcut: horizon_client::clipboard::EditShortcut) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if !crate::game::tconstruct_name_ui::focused(game) { return Continue(()); }
    let Some(pos) = game.container else { return Break(()); };
    let Some(edit) = game.world.chunks.__horizon_mixins_tconstruct_station_name_edit_mut().get_mut(&pos) else { return Break(()); };
    if edit.shortcut(shortcut, &mut game.clipboard) {
        let name = edit.value.clone();
        crate::game::tconstruct_name_ui::send(game, name);
    }
    Break(())
}

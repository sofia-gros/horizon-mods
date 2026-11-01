fn station_armor_drag(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if crate::game::tconstruct_ui::kind(game) != Some(true) { return Continue(()); }
    let Some(pos) = game.container else { return Continue(()); };
    let Some((angle, Some(last))) = game.world.chunks.__horizon_mixins_tconstruct_station_armor_preview_mut().get_mut(&pos) else { return Continue(()); };
    *angle += (game.cursor.0 - *last) / 10.0; *last = game.cursor.0;
    Break(())
}

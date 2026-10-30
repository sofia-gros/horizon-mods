fn clear_invalid_station_name(game: &mut crate::game::Game<'_>) {
    if crate::game::tconstruct_ui::kind(game) != Some(true) { return; }
    if let Some(pos) = game.container {
        if !crate::game::tconstruct_name_ui::available(game) {
            game.world.chunks.__horizon_mixins_tconstruct_station_name_edit_mut().remove(&pos);
        }
    }
}

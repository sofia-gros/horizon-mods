fn clear_invalid_station_name_after_click(value: (), game: &mut crate::game::Game<'_>) -> () {
    if game.remote.is_none() && crate::game::tconstruct_ui::kind(game) == Some(true) {
        if let Some(pos) = game.container {
            if !crate::game::tconstruct_name_ui::available(game) {
                game.world.chunks.__horizon_mixins_tconstruct_station_name_edit_mut().remove(&pos);
            }
        }
    }
    value
}

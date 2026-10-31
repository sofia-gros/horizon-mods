fn station_armor_release(game: &mut crate::game::Game<'_>) {
    if let Some(pos) = game.container {
        if let Some((_, dragging)) = game.world.chunks.__horizon_mixins_tconstruct_station_armor_preview_mut().get_mut(&pos) { *dragging = None; }
    }
}

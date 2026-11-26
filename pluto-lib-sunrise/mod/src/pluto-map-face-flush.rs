fn after_hud(value: (), game: &mut crate::game::Game<'_>, batch: &mut sunrise_client_render::hud::HudBatch) {
    game.draw_queued_map_faces(batch);
    value
}

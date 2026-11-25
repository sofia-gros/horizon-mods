fn after_hud(value: (), game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) {
    game.draw_queued_map_faces(batch);
    value
}

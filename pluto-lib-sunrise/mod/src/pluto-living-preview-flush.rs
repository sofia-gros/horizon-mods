fn after_screen_draw(value: (), game: &crate::game::Game<'_>, batch: &mut sunrise_client_render::hud::HudBatch) {
    game.draw_queued_living_previews(batch);
    value
}

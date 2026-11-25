fn after_screen_draw(value: (), game: &crate::game::Game<'_>, batch: &mut sunrise_client::hud::HudBatch) {
    game.draw_queued_living_previews(batch);
    value
}

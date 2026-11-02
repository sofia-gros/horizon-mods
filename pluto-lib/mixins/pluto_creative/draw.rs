fn pluto_draw(value: (), game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) {
    let pages = game.creative_screen.__horizon_mixins_pluto_creative_pages();
    if pages.count() > 1 {
        let (ox, oy) = crate::game::Game::creative_origin(batch);
        let atlas = &game.gui.atlas;
        for (x, text) in [(ox - 23.0, "<"), (ox + horizon_client::creative::Creative::WIDTH + 5.0, ">" )] {
            batch.fill(atlas, x, oy + 4.0, 18.0, 18.0, rgba(45, 45, 45, 230));
            batch.text(atlas, text, x + 6.0, oy + 9.0, u32::MAX, true);
        }
        batch.text(atlas, &format!("{}/{}", pages.page() + 1, pages.count()), ox - 22.0, oy + 25.0, u32::MAX, true);
    }
    value
}

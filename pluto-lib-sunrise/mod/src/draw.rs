fn pluto_draw(value: (), game: &mut crate::game::Game<'_>, batch: &mut sunrise_client_render::hud::HudBatch) {
    let (count, page) = sunrise_client_lib::pluto_paging::with(&game.creative_screen, |pages| (pages.count(), pages.page()));
    if count > 1 {
        let (ox, oy) = crate::game::Game::creative_origin(batch);
        let atlas = &game.gui.atlas;
        for (x, text) in [(ox - 23.0, "<"), (ox + sunrise_client_lib::creative::Creative::WIDTH + 5.0, ">" )] {
            batch.fill(atlas, x, oy + 4.0, 18.0, 18.0, rgba(45, 45, 45, 230));
            batch.text(atlas, text, x + 6.0, oy + 9.0, u32::MAX, true);
        }
        batch.text(atlas, &format!("{}/{}", page + 1, count), ox - 22.0, oy + 25.0, u32::MAX, true);
    }
    value
}

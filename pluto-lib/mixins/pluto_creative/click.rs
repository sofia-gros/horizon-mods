fn pluto_click(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return std::ops::ControlFlow::Continue(()); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let (ox, oy) = crate::game::Game::creative_origin(&batch);
    let (x, y) = game.cursor;
    let direction = if y >= oy + 4.0 && y < oy + 22.0 && x >= ox - 23.0 && x < ox - 5.0 { -1 }
        else if y >= oy + 4.0 && y < oy + 22.0 && x >= ox + horizon_client::creative::Creative::WIDTH + 5.0 && x < ox + horizon_client::creative::Creative::WIDTH + 23.0 { 1 }
        else { return std::ops::ControlFlow::Continue(()); };
    let screen = &mut game.creative_screen;
    let mut pages = std::mem::take(screen.__horizon_mixins_pluto_creative_pages_mut());
    if pages.count() <= 1 { *screen.__horizon_mixins_pluto_creative_pages_mut() = pages; return std::ops::ControlFlow::Continue(()); }
    if let Some((mut tabs, selected)) = pages.turn(direction, screen.selected) {
        for (i, tab) in tabs.iter_mut().enumerate() { tab.top = i < 7; tab.column = (i % 7) as u32; }
        screen.tabs = tabs;
        screen.select(selected);
        screen.forget_items();
    }
    *screen.__horizon_mixins_pluto_creative_pages_mut() = pages;
    std::ops::ControlFlow::Break(())
}

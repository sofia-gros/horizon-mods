fn pluto_click(game: &mut crate::game::Game<'_>) -> core::ops::ControlFlow<bool> {
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return std::ops::ControlFlow::Continue(()); };
    let batch = sunrise_client_render::hud::HudBatch::new(size);
    let (ox, oy) = crate::game::Game::creative_origin(&batch);
    let (x, y) = game.cursor;
    let direction = if y >= oy + 4.0 && y < oy + 22.0 && x >= ox - 23.0 && x < ox - 5.0 { -1 }
        else if y >= oy + 4.0 && y < oy + 22.0 && x >= ox + sunrise_client_lib::creative::Creative::WIDTH + 5.0 && x < ox + sunrise_client_lib::creative::Creative::WIDTH + 23.0 { 1 }
        else { return std::ops::ControlFlow::Continue(()); };
    let screen = &mut game.creative_screen;
    let mut pages = crate::pluto_paging::take(screen);
    if pages.count() <= 1 { crate::pluto_paging::put(screen, pages); return std::ops::ControlFlow::Continue(()); }
    if let Some((mut tabs, selected)) = pages.turn(direction, screen.selected) {
        for (i, tab) in tabs.iter_mut().enumerate() { tab.top = i < 7; tab.column = (i % 7) as u32; }
        screen.tabs = tabs;
        screen.select(selected);
        screen.forget_items();
    }
    crate::pluto_paging::put(screen, pages);
    core::ops::ControlFlow::Break(true)
}

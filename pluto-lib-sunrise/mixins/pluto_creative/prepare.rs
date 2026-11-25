fn pluto_prepare(game: &mut crate::game::Game<'_>) {
    let screen = &mut game.creative_screen;
    let mut pages = std::mem::take(screen.__horizon_mixins_pluto_creative_pages_mut());
    if pages.initialize(&screen.tabs) {
        screen.tabs = pages.visible();
        for (i, tab) in screen.tabs.iter_mut().enumerate() {
            tab.top = i < 7;
            tab.column = (i % 7) as u32;
        }
    }
    *screen.__horizon_mixins_pluto_creative_pages_mut() = pages;
}

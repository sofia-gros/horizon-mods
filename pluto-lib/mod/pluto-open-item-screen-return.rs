fn after_open_native_screen(
    value: (),
    game: &mut Game<'_>,
    key: Pos,
    window: i32,
    menu: &str,
    extra: Option<horizon_protocol::menu_extra::Message>,
) {
    let _ = value;
    if game.mod_screen.is_some() {
        return;
    }
    let Some(extra) = extra else { return };
    if !extra.data.starts_with(b"PLUTO\x01") {
        return;
    }
    let mut screen = None;
    game.native_screen(menu, &mut screen);
    if let Some(screen) = screen {
        game.mod_screen = Some(OpenNativeScreen { key, window, block: menu.to_owned(), screen, data: Vec::new(), extra: Some(extra) });
    }
}

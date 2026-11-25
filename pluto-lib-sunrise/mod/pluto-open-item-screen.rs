// A screen for a mod menu opened for an item (not a block), as a cancellable head hook: the
// body's own screen stands when the block at the stand-in position carries this menu (the
// return hook's guard left it, so the body runs), and the item screen is set here only on the
// path where the body would have left `mod_screen` cleared, skipping a body whose one other
// effect there is that clear. Every question asked is the body's own, of the same state.
fn before_open_native_screen(
    game: &mut Game<'_>,
    key: Pos,
    target: Option<Pos>,
    window: i32,
    menu: &str,
    extra: &Option<sunrise_protocol::menu_extra::Message>,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(extra) = extra.as_ref() else { return Continue(()) };
    if !extra.data.starts_with(b"PLUTO\x01") {
        return Continue(());
    }
    if let Some(t) = target {
        let block = game
            .res
            .facts()
            .get(game.world.block(t.0, t.1, t.2))
            .name
            .to_string();
        if !block.starts_with("minecraft:") {
            let mut container = None;
            game.native_block_entity_container(&block, &mut container);
            let opens = container.and_then(|c| mod_menu(c.menu.as_deref()?).map(|(m, _)| m));
            if opens == Some(menu) {
                let mut screen = None;
                game.native_screen(&block, &mut screen);
                if screen.is_some() {
                    return Continue(());
                }
            }
        }
    }
    let mut screen = None;
    game.native_screen(menu, &mut screen);
    if let Some(screen) = screen {
        game.mod_screen = Some(OpenNativeScreen { key, window, block: menu.to_owned(), screen, data: Vec::new(), extra: Some(extra.clone()) });
        return Break(());
    }
    Continue(())
}

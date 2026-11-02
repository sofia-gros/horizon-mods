fn tconstruct_crafting_tick(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    let Some((x, y, z)) = crate::game::tconstruct_crafting_station::position(game) else { return std::ops::ControlFlow::Continue(()); };
    let Some(eye) = game.eye_position() else { return std::ops::ControlFlow::Break(()); };
    let valid = &*game.res.facts().get(game.world.block(x, y, z)).name == "tconstruct:crafting_station";
    if !valid || !horizon_client::lan::guest::within_block_range(eye, game.reach() + 4.0, [x, y, z]) {
        game.close_container();
        game.screen = crate::game::Screen::Playing;
        game.capture(true);
    } else { crate::game::tconstruct_crafting_station::refresh(game); }
    std::ops::ControlFlow::Break(())
}

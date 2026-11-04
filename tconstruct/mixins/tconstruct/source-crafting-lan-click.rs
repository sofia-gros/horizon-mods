fn tconstruct_crafting_lan_click(game: &mut crate::game::Game<'_>, host: &mut LanHost, conn: u32, packet: &play::ContainerClick) -> std::ops::ControlFlow<bool> {
    match crate::game::lan_crafting::tconstruct_crafting_lan::click(game, host, conn, packet) {
        std::ops::ControlFlow::Break(value) => std::ops::ControlFlow::Break(value),
        std::ops::ControlFlow::Continue(()) => crate::game::lan_crafting::tconstruct_lan_craft_stale_guard::before_click(game, host, conn, packet),
    }
}

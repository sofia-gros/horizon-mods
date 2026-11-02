fn tconstruct_crafting_lan_click(game: &mut crate::game::Game<'_>, host: &mut LanHost, conn: u32, packet: &play::ContainerClick) -> std::ops::ControlFlow<bool> {
    crate::game::lan_crafting::tconstruct_crafting_lan::click(game, host, conn, packet)
}

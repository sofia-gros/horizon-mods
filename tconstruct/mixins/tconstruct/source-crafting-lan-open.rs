fn tconstruct_crafting_lan_open(game: &mut crate::game::Game<'_>, host: &mut LanHost, conn: u32, pos: [i32; 3]) -> std::ops::ControlFlow<()> {
    if &*game.res.facts().get(game.world.block(pos[0], pos[1], pos[2])).name != "tconstruct:crafting_station" { return std::ops::ControlFlow::Continue(()); }
    crate::game::lan_crafting::tconstruct_crafting_lan::open(game, host, conn, (pos[0], pos[1], pos[2]));
    std::ops::ControlFlow::Break(())
}

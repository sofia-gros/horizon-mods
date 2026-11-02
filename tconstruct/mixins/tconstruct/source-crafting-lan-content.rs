fn tconstruct_crafting_lan_content(game: &mut crate::game::Game<'_>,host: &mut LanHost,conn: u32) -> std::ops::ControlFlow<()> {
    crate::game::lan_crafting::tconstruct_crafting_lan::content(game,host,conn)
}

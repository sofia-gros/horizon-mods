fn tconstruct_crafting_lan_button(game: &mut crate::game::Game<'_>,host: &mut crate::game::lan::LanHost,conn: u32,window: i32,id: i32) -> std::ops::ControlFlow<()> {
    crate::game::lan_crafting::tconstruct_crafting_lan::scroll(game,host,conn,window,id)
}

// The end of a player's break of the block at `pos` (the helper runs as `break_block_as_player` returns, the block removed or not): the mods hear
// whether the block went, with the game as the host. The result of the break is passed through unchanged.
fn pluto_block_break(value:crate::Result<bool>,game:&mut crate::game::Game<'_>,pos:[i32;3])->crate::Result<bool> {
    if let Some(sink)=crate::entry::pluto_block_break_access::sink() {
        (sink.after)(game,pos,matches!(value,Ok(true)));
    }
    value
}

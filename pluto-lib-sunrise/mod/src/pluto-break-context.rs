// Reads the mined block's block entity at the head of the game's own break, before the block is gone,
// so the break context can answer the block-entity NBT tests of the off hand's place tag. The bridge's
// own block-entity read names mod block entities only; the game's own `block_entity_nbt` reads any block
// entity, as its adventure-mode matcher does.
use super::*;

pub(crate) fn snapshot(game: &mut Game<'_>, pos: [i32; 3]) {
    if !game.native_authoritative() {
        return;
    }
    let dimension = game.dim.id();
    let nbt = game.block_entity_nbt(pos);
    crate::pluto_break_context_access::note_block_entity_nbt(&dimension, pos, nbt);
}

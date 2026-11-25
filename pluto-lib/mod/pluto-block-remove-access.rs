// The removal the game's block entity removal reports (pluto_block_remove): the mod-facing host has no
// call that runs when a block entity is removed, so the head of the game's own removal reports it through
// this sink, and the bridge hears the removal while the block entity is still in the world, answering the
// stacks its type keeps outside its slots for the game to drop at the position.
use std::sync::OnceLock;

/// A mod block entity being removed with its side effects: the block at `pos` in `dimension` is going.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRemoved {
    pub dimension: String,
    pub pos: [i32; 3],
    pub block: String,
}

/// What the bridge does with a removal: runs the block entity type's removal hook and answers the
/// stacks to drop at the position (what the type keeps outside its item slots).
#[derive(Clone, Copy)]
pub struct BlockRemoveSink {
    pub removed: fn(&mut dyn crate::mod_api::Host, BlockRemoved) -> Vec<crate::items::ItemStack>,
}

static SINK: OnceLock<BlockRemoveSink> = OnceLock::new();

/// Installs the sink the bridge hands the seam; false when one is already installed.
pub fn set_block_remove_sink(sink: BlockRemoveSink) -> Result<(), &'static str> {
    SINK.set(sink).map_err(|_| "a block removal sink is already installed")
}

/// The installed sink, if any.
pub fn block_remove_sink() -> Option<BlockRemoveSink> {
    SINK.get().copied()
}

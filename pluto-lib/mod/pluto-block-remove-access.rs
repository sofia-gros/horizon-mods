// The removal the game's block entity removal reports (pluto_block_remove): the mod-facing host has no
// call that runs when a block entity is removed, and the removal itself runs inside the game's level
// operations, where no host exists to call. So the head of the game's own removal captures the removal
// into this module's queue — the block entity's saved data and item slots, copied while it is still in
// the world — and the head of the game's spill flush, which ends every level operation, drains the
// queue through the sink the bridge installed: the bridge runs the block entity type's removal hook
// over the captured data, answering the stacks to drop at the position.
use std::sync::{Mutex, OnceLock};

/// A mod block entity removed with its side effects: the block at `pos` in `dimension` went, and
/// `nbt` and `items` are its saved data and item slots as they were when its removal began.
#[derive(Clone)]
pub struct BlockRemoved {
    pub dimension: String,
    pub pos: [i32; 3],
    pub block: String,
    pub nbt: horizon_world::nbt::Compound,
    pub items: Vec<Option<crate::items::ItemStack>>,
}

/// What the bridge does with a removal: runs the block entity type's removal hook over the captured
/// data and answers the stacks to drop at the position (what the type keeps outside its item slots).
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

/// Removals captured at the head of the game's block entity removal, waiting for the spill
/// flush that ends their level operation to drain them. A capture is drained by the flush of
/// the operation that captured it, in the dimension it was captured in; the queue only ever
/// holds one operation's removals, and the cap keeps a flush that never comes (a world
/// closing mid-operation) from holding more than this.
static QUEUE: Mutex<Vec<BlockRemoved>> = Mutex::new(Vec::new());

const QUEUE_CAP: usize = 4096;

/// Queues a captured removal for the drain at the end of its level operation.
pub fn stash_removal(removed: BlockRemoved) {
    let mut queue = QUEUE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if queue.len() >= QUEUE_CAP {
        let excess = queue.len() + 1 - QUEUE_CAP;
        queue.drain(..excess);
    }
    queue.push(removed);
}

/// Takes the queued removals of one dimension, leaving the rest queued for their own
/// dimension's next flush.
pub fn take_removals_for(dimension: &str) -> Vec<BlockRemoved> {
    let mut queue = QUEUE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut taken = Vec::new();
    let mut kept = Vec::new();
    for removed in queue.drain(..) {
        if removed.dimension == dimension {
            taken.push(removed);
        } else {
            kept.push(removed);
        }
    }
    *queue = kept;
    taken
}

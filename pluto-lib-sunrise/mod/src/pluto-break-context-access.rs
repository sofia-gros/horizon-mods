// The mined block's block entity as a break started, for the break context: the bridge's own block-entity
// read names mod block entities only, so a vanilla one (a chest, a furnace) is read here, at the head of
// the game's own break, before the block is gone, and the bridge takes it when it builds the context.
// Entries are consumed on take and capped, so a break that never builds a context leaves nothing stale
// behind for the next break at the same position: every head seam overwrites the entry first.
use std::sync::Mutex;

/// One noted block entity, in plain data: the dimension and position it was read at, and its saved
/// compound (`BlockEntity.saveWithFullMetadata`), or nothing when the block has no block entity.
#[derive(Clone, Debug)]
struct Noted {
    dimension: String,
    pos: [i32; 3],
    nbt: Option<sunrise_core::nbt::Tag>,
}

static NOTED: Mutex<Vec<Noted>> = Mutex::new(Vec::new());

/// Notes the block entity at `pos` in `dimension` for the break context to take; an entry for the same
/// position is replaced, and past 64 entries the oldest goes.
pub fn note_block_entity_nbt(dimension: &str, pos: [i32; 3], nbt: Option<sunrise_core::nbt::Tag>) {
    let Ok(mut noted) = NOTED.lock() else { return };
    noted.retain(|entry| !(entry.dimension == dimension && entry.pos == pos));
    if noted.len() >= 64 {
        noted.remove(0);
    }
    noted.push(Noted { dimension: dimension.to_owned(), pos, nbt });
}

/// Takes the noted block entity at `pos` in `dimension`, if the head seam noted one: the saved compound,
/// or nothing when the block had no block entity. Nothing when the seam did not run.
pub fn take_block_entity_nbt(dimension: &str, pos: [i32; 3]) -> Option<sunrise_core::nbt::Tag> {
    let Ok(mut noted) = NOTED.lock() else { return None };
    let at = noted.iter().position(|entry| entry.dimension == dimension && entry.pos == pos)?;
    noted.remove(at).nbt
}

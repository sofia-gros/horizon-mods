// Transport for a mod's request to brush a block: the API side queues the request (the
// mod-facing host has no brush call; the game's own `BrushableBlockEntity.brush` lives
// inside the game), and the game's creature tick seam applies the queued strokes in the
// level in place (see pluto-brush-flush in the game's brush module).
use std::sync::Mutex;

/// One queued brush stroke, in plain data: the block brushed and the face brushed
/// (a unit offset), which the block entity remembers as the face its item shows from.
#[derive(Clone, Debug)]
pub struct Queued {
    pub dimension: String,
    pub pos: [i32; 3],
    pub face: [i32; 3],
}

static QUEUE: Mutex<Vec<Queued>> = Mutex::new(Vec::new());

/// Queues a brush stroke for the seam in its dimension; false when the queue is full (4096),
/// as the explosion queue refuses past its bound rather than growing without limit.
pub fn push(dimension: &str, pos: [i32; 3], face: [i32; 3]) -> bool {
    let Ok(mut queue) = QUEUE.lock() else { return false };
    if queue.len() >= 4096 { return false; }
    queue.push(Queued { dimension: dimension.to_owned(), pos, face });
    true
}

/// Takes the queued brush strokes for `dimension` (the level in place), leaving the others queued.
pub fn take_for(dimension: &str) -> Vec<Queued> {
    let Ok(mut queue) = QUEUE.lock() else { return Vec::new() };
    let mut mine = Vec::new();
    let mut rest = Vec::new();
    for queued in std::mem::take(&mut *queue) {
        if queued.dimension == dimension { mine.push(queued); } else { rest.push(queued); }
    }
    *queue = rest;
    mine
}

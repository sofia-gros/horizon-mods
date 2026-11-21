// Transport for a mod's request to set off an explosion: the API side queues the request (the
// mod-facing host has no explosion call; the game's own `native_explode` lives inside the game),
// and the game's mob-event seam sets the queued blasts off in the level in place (see
// pluto-explosion-flush in the game's potions module).
use std::sync::Mutex;

/// What the blast does to the blocks it reaches, in the game's own terms
/// (`Level.ExplosionInteraction`, as `native_explode`'s blast kinds name them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blast {
    /// Blocks stay; entities are hurt and pushed.
    Keep,
    /// Blocks break and drop by the block-explosion rule.
    Block,
    /// Blocks break and drop by the tnt-explosion rule.
    Tnt,
}

/// One queued explosion, in plain data.
#[derive(Clone, Debug)]
pub struct Queued {
    pub dimension: String,
    pub center: [f64; 3],
    pub radius: f32,
    pub fire: bool,
    pub blast: Blast,
}

static QUEUE: Mutex<Vec<Queued>> = Mutex::new(Vec::new());

/// Queues an explosion for the seam in its dimension; false when the queue is full (4096), as the
/// contact queue refuses past its bound rather than growing without limit.
pub fn push(dimension: &str, center: [f64; 3], radius: f32, fire: bool, blast: Blast) -> bool {
    let Ok(mut queue) = QUEUE.lock() else { return false };
    if queue.len() >= 4096 { return false; }
    queue.push(Queued { dimension: dimension.to_owned(), center, radius, fire, blast });
    true
}

/// Takes the queued explosions for `dimension` (the level in place), leaving the others queued.
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

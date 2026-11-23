// Transport for a mod's request to end a player's item use: the API side queues the
// request (the mod-facing host has no call that ends a use; the game's own
// `LivingEntity.releaseUsingItem` and `stopUsingItem` live inside the game), and the
// game's creature tick seam applies the queued end to the use in progress (see
// pluto-use-stop-flush in the game's using_item module).
//
// A request is stamped with the generation of the use it was queued during: the API
// side counts a player's item-use starts (begin_use), so a request whose use has
// since ended — and another begun — is stale and dropped at the seam, never applied
// to the newer use. A request for a use that simply ended is dropped the same way,
// as the seam finds no use in progress then. One limit stays: the generation counts
// the starts of mod stack uses (the only uses a request can be queued during), so a
// request still queued when its use ends could in principle meet a vanilla use the
// player began before the seam runs; the window is the rest of that tick, and the
// seam drains the queue every tick.
use std::collections::HashMap;
use std::sync::Mutex;

/// How a use ends, as the game's own ends split: a release runs the use's
/// releaseUsing first (a mod's Stop may still skip that part, never the end
/// itself), a stop only ends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Release,
    Stop,
}

/// One queued use end, in plain data: whose use, how it ends, and the generation
/// of the use it was ordered during.
#[derive(Clone, Debug)]
pub struct Queued {
    pub player: u128,
    pub end: End,
    pub generation: u64,
}

struct State {
    queue: Vec<Queued>,
    generations: HashMap<u128, u64>,
}

static STATE: Mutex<State> = Mutex::new(State { queue: Vec::new(), generations: HashMap::new() });

/// Counts a new item use of `player` beginning: requests queued during their
/// earlier uses go stale. A start the game then refuses still counts — no use
/// ticks follow it, so nothing is ever stamped with its generation.
pub fn begin_use(player: u128) {
    let Ok(mut state) = STATE.lock() else { return };
    *state.generations.entry(player).or_default() += 1;
}

/// Queues the end of `player`'s use in progress, stamped with its generation;
/// false when the queue is full (4096), as the brush queue refuses past its bound
/// rather than growing without limit.
pub fn push(player: u128, end: End) -> bool {
    let Ok(mut state) = STATE.lock() else { return false };
    if state.queue.len() >= 4096 { return false; }
    let generation = state.generations.get(&player).copied().unwrap_or(0);
    state.queue.push(Queued { player, end, generation });
    true
}

/// Takes the queued ends of `player`'s current use (the generation in place),
/// leaving the other players' queued. Their own stale ends — ordered for an
/// earlier use — are taken and discarded here, never carried into a later use.
pub fn take_for(player: u128) -> Vec<Queued> {
    let Ok(mut state) = STATE.lock() else { return Vec::new() };
    let current = state.generations.get(&player).copied().unwrap_or(0);
    let mut mine = Vec::new();
    let mut rest = Vec::new();
    for queued in std::mem::take(&mut state.queue) {
        if queued.player != player { rest.push(queued); }
        else if queued.generation == current { mine.push(queued); }
    }
    state.queue = rest;
    mine
}

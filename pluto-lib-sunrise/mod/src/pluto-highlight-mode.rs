// The local player's game mode, as the game's mode setter last left it: the highlight transport reads it for the frame's block reach, since no host read states the local player's mode on the render path (see the head injection). 0 survival, 1 creative, 2 adventure, 3 spectator, as the game numbers its modes; survival before any mode is set.
use std::sync::atomic::{AtomicI32, Ordering};

static MODE: AtomicI32 = AtomicI32::new(0);

pub fn set_mode(mode: i32) {
    MODE.store(mode, Ordering::SeqCst);
}

pub fn mode() -> i32 {
    MODE.load(Ordering::SeqCst)
}

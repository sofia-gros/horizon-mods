// Transport for a mod's request to move a named player's arm or broadcast that their stack broke: the
// API side queues the request (the mod-facing host has no swing or break-broadcast call; the game's own
// swing and break handling live inside the game), and the game's break-completion seam and player-tick
// seam play the queued requests for the player acting then (see pluto-player-animate-flush).
use std::sync::Mutex;

/// One queued arm swing, in plain data: the player to swing, and which arm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwingRequest {
    pub player: u128,
    pub off_hand: bool,
}

/// One queued item-break broadcast, in plain data: the player whose stack broke, the slot it broke in,
/// and the stack as it broke (for the break sound and the crumbs).
#[derive(Clone, Debug)]
pub struct BreakRequest {
    pub player: u128,
    pub slot: crate::mod_api::EquipmentSlot,
    pub stack: crate::items::ItemStack,
}

static SWINGS: Mutex<Vec<SwingRequest>> = Mutex::new(Vec::new());
static BREAKS: Mutex<Vec<BreakRequest>> = Mutex::new(Vec::new());

/// Queues an arm swing for the player; false when the queue is full (4096), as the contact queue refuses
/// past its bound rather than growing without limit.
pub fn push_swing(player: u128, off_hand: bool) -> bool {
    let Ok(mut queue) = SWINGS.lock() else { return false };
    if queue.len() >= 4096 {
        return false;
    }
    queue.push(SwingRequest { player, off_hand });
    true
}

/// Queues an item-break broadcast for the player; false when the queue is full (4096).
pub fn push_break(
    player: u128,
    slot: crate::mod_api::EquipmentSlot,
    stack: crate::items::ItemStack,
) -> bool {
    let Ok(mut queue) = BREAKS.lock() else { return false };
    if queue.len() >= 4096 {
        return false;
    }
    queue.push(BreakRequest { player, slot, stack });
    true
}

/// Takes the queued swings and break broadcasts for `player`, leaving the others queued.
pub fn take_for_player(player: u128) -> (Vec<SwingRequest>, Vec<BreakRequest>) {
    let mut swings = Vec::new();
    let mut breaks = Vec::new();
    if let Ok(mut queue) = SWINGS.lock() {
        let (mine, rest) =
            std::mem::take(&mut *queue).into_iter().partition(|request| request.player == player);
        swings = mine;
        *queue = rest;
    }
    if let Ok(mut queue) = BREAKS.lock() {
        let (mine, rest) =
            std::mem::take(&mut *queue).into_iter().partition(|request| request.player == player);
        breaks = mine;
        *queue = rest;
    }
    (swings, breaks)
}

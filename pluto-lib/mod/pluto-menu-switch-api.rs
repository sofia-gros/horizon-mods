// Menu switches a mod's menu button asked for during its press (a tabbed table's tab asks to
// open the neighbouring table's menu), waiting for the flush at the end of that press
// (pluto-menu-switch): the press itself runs through the mod API, which has no game at hand,
// while opening a menu is the game's own work.
use std::sync::Mutex;

/// One queued switch: the menu open at `from` is to close and the menu of the block at `to`
/// is to open for the player who pressed the button, keeping what they carry.
pub struct QueuedSwitch {
    pub from: [i32; 3],
    pub to: [i32; 3],
}

static QUEUE: Mutex<Vec<QueuedSwitch>> = Mutex::new(Vec::new());

/// Queues a switch for the flush at the end of the current menu button press.
pub fn queue(from: [i32; 3], to: [i32; 3]) {
    if let Ok(mut queue) = QUEUE.lock() {
        queue.push(QueuedSwitch { from, to });
    }
}

/// Takes every queued switch, leaving the queue empty.
pub fn take_queued() -> Vec<QueuedSwitch> {
    QUEUE.lock().map(|mut queue| std::mem::take(&mut *queue)).unwrap_or_default()
}

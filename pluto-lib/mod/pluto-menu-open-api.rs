// Menu opens a mod's block use asked for during its right-click dispatch (pluto-menu-open):
// clicking a block whose block entity declares a menu asks for that menu to open, and the
// ask waits in this queue to be settled in the host's right-click bridge right after
// the dispatch (pluto-menu-open-settle), because the dispatch runs through the mod API,
// which has no game at hand, while opening a menu is the game's own work — and only a
// click nothing else claimed may open one.
use std::sync::Mutex;

/// One queued open: the menu of the block at `pos` is to open for `player` (the click's
/// actor; `None` is taken by the next settle that runs, as in pluto-menu-switch).
/// `ready` is set when a guest's click is settled in the bridge with nothing else
/// claiming it: a guest's open is applied by the guest flush at the end of the guest's
/// turn (pluto-menu-open-guest-flush) through the host's own guest container open,
/// while the local player's opens are applied by the settle in the same click.
pub struct QueuedOpen {
    pub pos: [i32; 3],
    pub player: Option<u128>,
    pub ready: bool,
}

static QUEUE: Mutex<Vec<QueuedOpen>> = Mutex::new(Vec::new());

/// Queues a menu open for the flush at the end of the current click.
pub fn queue_open(pos: [i32; 3], player: Option<u128>) {
    if let Ok(mut queue) = QUEUE.lock() {
        queue.push(QueuedOpen { pos, player, ready: false });
    }
}

/// Takes the queued opens for `player` (playerless ones go to whoever flushes first),
/// leaving the rest queued.
pub fn take_queued_for(player: u128) -> Vec<QueuedOpen> {
    let Ok(mut queue) = QUEUE.lock() else { return Vec::new() };
    let (mine, rest) = std::mem::take(&mut *queue).into_iter().partition(|open| open.player.is_none_or(|uuid| uuid == player));
    *queue = rest;
    mine
}

/// Whether an open for `pos` is queued for `player`.
pub fn has_queued_for(player: u128, pos: [i32; 3]) -> bool {
    let Ok(queue) = QUEUE.lock() else { return false };
    queue.iter().any(|open| open.pos == pos && open.player.is_none_or(|uuid| uuid == player))
}

/// Marks `player`'s queued open for `pos` ready: its click ended unconsumed on a guest's
/// turn, so the guest flush at the end of the turn may apply it.
pub fn mark_ready_for(player: u128, pos: [i32; 3]) {
    if let Ok(mut queue) = QUEUE.lock() {
        for open in queue.iter_mut() {
            if open.pos == pos && open.player.is_none_or(|uuid| uuid == player) {
                open.ready = true;
            }
        }
    }
}

/// Drops `player`'s queued opens that are not ready: their click was consumed after all,
/// so no menu opens for it.
pub fn drop_unready_for(player: u128) {
    if let Ok(mut queue) = QUEUE.lock() {
        queue.retain(|open| open.ready || !open.player.is_none_or(|uuid| uuid == player));
    }
}

/// Takes every queued open that is ready, leaving the rest queued.
pub fn take_all_ready() -> Vec<QueuedOpen> {
    let Ok(mut queue) = QUEUE.lock() else { return Vec::new() };
    let (ready, rest) = std::mem::take(&mut *queue).into_iter().partition(|open| open.ready);
    *queue = rest;
    ready
}

#[cfg(test)]
mod tests {
    use super::{drop_unready_for, has_queued_for, mark_ready_for, queue_open, take_all_ready, take_queued_for};

    #[test]
    fn queued_opens_go_to_their_own_flush() {
        queue_open([1, 2, 3], Some(7));
        queue_open([4, 5, 6], Some(8));
        // A flush for a player nothing is addressed to takes nothing.
        assert!(take_queued_for(9).is_empty());
        // A playerless open goes to whoever flushes first.
        queue_open([7, 8, 9], None);
        let mine = take_queued_for(7);
        assert_eq!(mine.len(), 2);
        assert!(mine.iter().any(|open| open.pos == [1, 2, 3]));
        assert!(mine.iter().any(|open| open.pos == [7, 8, 9]));
        // Only player 8's open remains. The guest cycle: it is not ready until the
        // click that asked for it ends unconsumed, and only ready ones are taken.
        assert!(has_queued_for(8, [4, 5, 6]));
        assert!(!has_queued_for(8, [1, 2, 3]));
        assert!(!has_queued_for(7, [4, 5, 6]));
        assert!(take_all_ready().is_empty());
        mark_ready_for(8, [4, 5, 6]);
        let ready = take_all_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].pos, [4, 5, 6]);
        // A consumed click's opens are dropped, never opened later.
        queue_open([10, 11, 12], Some(8));
        drop_unready_for(8);
        assert!(take_queued_for(8).is_empty());
    }
}

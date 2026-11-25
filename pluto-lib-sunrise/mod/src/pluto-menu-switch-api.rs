// Menu switches a mod's menu button asked for during its press (a tabbed table's tab asks to
// open the neighbouring table's menu), waiting for the flush at the end of that press
// (pluto-menu-switch): the press itself runs through the mod API, which has no game at hand,
// while opening a menu is the game's own work.
use std::sync::Mutex;

/// One queued switch: the menu open at `from` is to close and the menu of the block at `to`
/// is to open for the player who pressed the button, keeping what they carry. `player` is the
/// presser's uuid when the press knew it (the local player, or a LAN guest), and None for a
/// switch queued without one: the flushes take only their own player's switches, so a guest's
/// switch is never dropped by the local player's flush.
pub struct QueuedSwitch {
    pub from: [i32; 3],
    pub to: [i32; 3],
    pub player: Option<u128>,
}

static QUEUE: Mutex<Vec<QueuedSwitch>> = Mutex::new(Vec::new());

/// Queues a switch for the flush at the end of the current menu button press.
pub fn queue(from: [i32; 3], to: [i32; 3], player: Option<u128>) {
    if let Ok(mut queue) = QUEUE.lock() {
        queue.push(QueuedSwitch { from, to, player });
    }
}

/// Takes every queued switch, leaving the queue empty.
pub fn take_queued() -> Vec<QueuedSwitch> {
    QUEUE.lock().map(|mut queue| std::mem::take(&mut *queue)).unwrap_or_default()
}

/// Takes the queued switches for `player`, leaving the rest queued: the local flush takes only
/// the local player's, so a guest's switch survives until the guest flush runs; a switch with
/// no player is taken by any flush (the legacy queue).
pub fn take_queued_for(player: u128) -> Vec<QueuedSwitch> {
    QUEUE.lock().map(|mut queue| {
        let mut taken = Vec::new();
        let mut index = 0;
        while index < queue.len() {
            if queue[index].player.map_or(true, |who| who == player) {
                taken.push(queue.remove(index));
            } else {
                index += 1;
            }
        }
        taken
    }).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taking_one_players_switches_leaves_the_other_players_queued() {
        queue([0, 0, 0], [1, 0, 0], Some(7));
        queue([0, 0, 0], [2, 0, 0], Some(9));
        queue([0, 0, 0], [3, 0, 0], None);
        let taken = take_queued_for(7);
        assert_eq!(taken.len(), 2, "the player's own switch and the playerless one");
        assert_eq!(taken.iter().map(|switch| switch.to).collect::<Vec<_>>(), [[1, 0, 0], [3, 0, 0]]);
        assert_eq!(take_queued().iter().map(|switch| switch.to).collect::<Vec<_>>(), [[2, 0, 0]], "the other player's switch survives");
        assert!(take_queued().is_empty());
    }
}

// Plays the arm swings and item-break broadcasts the mods queued (pluto_player_animate_access) for the
// player acting now: a swing through the game's own swing (the arm moves on their screen and, for a guest,
// the players around are told through the guest swing count the turn already attributes), a break through
// the game's own equipment-break handling (the statistic, the break sound and crumbs where they stand, and
// the item-break hook for a guest). A request for another player waits for their turn: the swing and the
// break both belong to the player whose turn it is, as the guest machinery reads them from the acting state.
use super::*;
use sunrise_client::first_person::{Hand, SwingAnimation};

pub(crate) fn flush(game: &mut Game<'_>) {
    if !game.native_authoritative() {
        return;
    }
    let player = sunrise_client::lan::playerdata::ints_uuid(game.acting_player_uuid());
    let (swings, breaks) = crate::entry::pluto_player_animate_access::take_for_player(player);
    for swing in swings {
        let hand = if swing.off_hand { Hand::Off } else { Hand::Main };
        game.swing_hand(hand, SwingAnimation::DEFAULT);
    }
    for broke in breaks {
        game.player_equipment_broke(broke.slot, &broke.stack);
    }
}

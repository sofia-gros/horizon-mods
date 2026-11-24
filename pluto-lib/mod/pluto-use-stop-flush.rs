// Applies the item-use ends the mods queued (pluto_use_stop_access) to the use in
// progress of the player this game acts for, as the game's own ends land them: a
// release runs the use's release first (releaseUsingItem, with its Stop event and
// its release dispatch, as letting go of the button runs it), a stop only ends the
// use (stopUsingItem, with its finish vibration). An end whose use already ended —
// the seam finds nothing in progress — does nothing, and neither does a second end
// of the same batch once the first has ended the use. Runs after the brush flush
// in the same seam: a stroke the use queued lands before the end that follows it,
// as a stroke lands before the release its own tick orders in the original.
use super::*;

pub(crate) fn flush(game: &mut Game<'_>) {
    if !game.native_authoritative() { return; }
    let player = crate::lan::playerdata::ints_uuid(game.acting_player_uuid());
    for queued in crate::entry::pluto_use_stop_access::take_for(player) {
        let Some((slot, ticks)) = game.using else { continue };
        match queued.end {
            crate::entry::pluto_use_stop_access::End::Release => {
                let Some(stack) = game.stack_in(slot).cloned() else { game.using = None; continue };
                game.release_using_item(&stack, ticks);
            }
            crate::entry::pluto_use_stop_access::End::Stop => game.stop_using(),
        }
    }
}

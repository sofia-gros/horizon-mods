// Runs the break's AFTER inside the break: the woven return of the game's own break (see
// pluto-break-complete-return) reports the completed break through the sink the bridge installed, so
// the mods hear the break before the removal is sent to the players around. The sink is the only way
// back into the bridge: the game cannot call the mod crate directly.
use super::*;

pub(crate) fn dispatch(game: &mut Game<'_>, pos: [i32; 3]) {
    if !game.native_authoritative() {
        return;
    }
    let Some(sink) = crate::entry::pluto_break_complete_access::break_complete_sink() else {
        return;
    };
    let completed =
        crate::entry::pluto_break_complete_access::BreakCompleted { dimension: game.dim.id().to_owned(), pos };
    (sink.completed)(game, completed);
}

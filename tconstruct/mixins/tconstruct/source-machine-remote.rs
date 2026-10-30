// The server's chest menu uses a synthetic client container position; the
// targeted world block remains available in the existing remote use state.
use super::*;
pub(in crate::game) fn position(game: &Game<'_>) -> Option<(i32, i32, i32)> {
    let [x, y, z] = game.remote.as_ref()?.use_target?;
    Some((x, y, z))
}

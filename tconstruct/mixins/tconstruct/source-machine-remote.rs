// The server's chest menu uses a synthetic client container position; the
// targeted world block remains available in the existing remote use state.
use super::*;
pub(in crate::game) fn position(game: &Game<'_>) -> Option<(i32, i32, i32)> {
    let [x, y, z] = game.remote.as_ref()?.use_target?;
    Some((x, y, z))
}

/// Update the remote target only after the server acknowledges a tab switch.
pub(in crate::game) fn set_position(game: &mut Game<'_>, pos: (i32, i32, i32)) {
    if let Some(remote) = game.remote.as_mut() { remote.use_target = Some([pos.0, pos.1, pos.2]); }
}

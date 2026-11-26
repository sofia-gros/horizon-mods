// Lives under remote.rs so the native heard-level authority stays private.
// HEAD helpers call before any consumer callback can observe a leaving lifetime.
// Source-integrated only: native attachment/lifecycle verification is still pending.
use super::*;
use crate::pluto_client_block_entity_state::client_level_replaced;

pub(crate) fn before_replace(game: &Game<'_>, keep_world: bool, incoming: &str) {
    let Some(remote) = game.remote.as_ref() else { return };
    if client_level_replaced(keep_world, remote.heard_level.as_deref(), incoming) {
        crate::game::pluto_client_block_entity_tick::reset_client_level(game);
    }
}

pub(crate) fn before_unload(game: &Game<'_>) {
    // setLevel creates/notifies the new level BEFORE unloading the old one.
    // Its heard_level is already Some(new), so never reset that new lifetime.
    // Disconnect/reconfiguration take heard_level before this notification;
    // those paths can retain the old world until shutdown or the next login.
    if game.remote.as_ref().is_some_and(|remote| remote.heard_level.is_none()) {
        crate::game::pluto_client_block_entity_tick::reset_client_level(game);
    }
}

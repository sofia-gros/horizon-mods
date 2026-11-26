// Experimental adapter. There is deliberately no native activation, network
// subscription, send or document-mutation entry. Never infer ownership from
// the current book. This experiment requires the frozen current-book service.
use super::Game;
use sunrise_client::chunk_map::pluto_guide_lifetime_state::{with_session, with_session_mut, Opening, Request};

fn session_key(game: &Game<'_>) -> usize {
    std::ptr::from_ref(&game.world.chunks) as usize
}

pub(crate) fn invalidate(game: &mut Game<'_>) {
    if game.options.execution_role.is_client() && game.lan_acting.is_none() {
        with_session_mut(session_key(game), |state| state.invalidate());
    }
}
/// Observe the existing state only. A current book never creates an opening.
pub(crate) fn active(game: &Game<'_>, opening: Opening) -> bool {
    game.options.execution_role.is_client()
        && game.lan_acting.is_none()
        && with_session(session_key(game), |state| state.active(opening))
}
/// A conditional correlation experiment, not a screen-update capability.
/// The existing current-book gate/converter is reused without a page action.
pub(crate) fn admit_reply(
    game: &mut Game<'_>,
    request: Request,
    owner: &str,
    channel: &str,
) -> bool {
    let epoch = game
        .remote
        .as_ref()
        .filter(|remote| remote.has_level())
        .and_then(|remote| remote.native_epoch())
        .map(|epoch| epoch.get());
    let admitted = super::pluto_current_book::with_current(game, |_, _| ()).is_ok();
    with_session_mut(session_key(game), |state| {
        state.accept(request, epoch, admitted, owner, channel)
    })
}

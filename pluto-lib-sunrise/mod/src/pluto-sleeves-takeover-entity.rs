// After the entity interaction returns: the chestplate takeover (Tinkers'
// InteractionHandler.afterEntityInteract, at its LOWEST priority, after the entity's own
// interaction ran) runs the worn sleeves module when a click on an entity took nothing.
// Only a guest's entity packet ends here: the local player's click falls through to the
// hand's own use when the interaction takes nothing, and that seam reports the same
// click, so this one stays quiet there. The game reports the interaction's outcome; the
// pure side owns the piece, the selection and the throw.
//
// The seam is a cancellable HEAD, not a RETURN: the interaction's body works through
// closures and `?`, which a RETURN cannot plan (E_UNSUPPORTED_CONTROL_FLOW). The HEAD
// runs the interaction itself under a guard (the nested call's HEAD passes through to
// the vanilla body), then reports the outcome the interaction leaves and breaks with
// it, so the body runs exactly once and the report reads the game as the interaction
// leaves it, as the return seam's report did.
use std::cell::Cell;

use super::*;

use mod_api::Host;

thread_local! {
    /// Whether the seam's own call is running: its nested HEAD passes through to the
    /// vanilla body, so the interaction runs exactly once.
    static IN_CALL: Cell<bool> = const { Cell::new(false) };
}

/// Marks the seam's own call for its duration; the mark clears on drop, so a panic in
/// the interaction cannot leave the seam passing through.
struct InCall;

impl InCall {
    fn enter() -> Self {
        IN_CALL.with(|slot| slot.set(true));
        InCall
    }
}

impl Drop for InCall {
    fn drop(&mut self) {
        IN_CALL.with(|slot| slot.set(false));
    }
}

/// The seam on the entity interaction. The outermost call runs the interaction itself:
/// the nested call that makes passes through to the vanilla body, so the body runs
/// exactly once, and the outcome it leaves is reported through `after_entity_use`
/// before the seam breaks with it — the caller sees the interaction's own result in
/// every case, as it did from the return seam. The mark is down again by the time the
/// report runs, so an interaction the provider's answer sets off is entered fresh and
/// reported the same way. An errored interaction reports nothing (the report's own
/// gate), so it just breaks with the error.
pub(crate) fn interact(
    game: &mut Game<'_>,
    target: Option<InteractTarget>,
) -> std::ops::ControlFlow<Result<bool>> {
    use std::ops::ControlFlow::{Break, Continue};
    if IN_CALL.with(Cell::get) {
        return Continue(());
    }
    let value = {
        let _seam = InCall::enter();
        game.interact_entity(target)
    };
    match value {
        Ok(took) => {
            after_entity_use(game, Ok(took));
            Break(Ok(took))
        }
        Err(error) => Break(Err(error)),
    }
}

/// The look direction for a yaw and pitch in degrees (the host's own shoot-from-rotation
/// formula: the yaw and pitch are converted to radians, then the direction is
/// `[-sin(yaw) * cos(pitch), -sin(pitch), cos(yaw) * cos(pitch)]`).
fn look_direction(yaw: f32, pitch: f32) -> [f64; 3] {
    let (yaw, pitch) = (yaw.to_radians(), pitch.to_radians());
    [
        f64::from(-yaw.sin() * pitch.cos()),
        f64::from(-pitch.sin()),
        f64::from(yaw.cos() * pitch.cos()),
    ]
}

/// Runs the post-interaction takeover for an entity click the game did not consume:
/// nothing happens when the interaction consumed the click (or failed), when the click
/// did not arrive as an entity packet, when the hand that clicked still holds something,
/// when the player is a spectator, when the eye cannot be read, or when no mod listens.
/// The hand is read as the interaction leaves it; a non-consuming interaction does not
/// fill an empty hand. The cooldown surface reads the game this call runs in, held for
/// the provider call only.
pub(crate) fn after_entity_use(game: &mut Game<'_>, value: Result<bool>) {
    if !matches!(value, Ok(false)) {
        return;
    }
    if !matches!(game.guest_use, Some(GuestUse::Entity(..))) {
        return;
    }
    let hand_empty = match game.hand {
        Hand::Main => game.held().is_none(),
        Hand::Off => game.offhand.is_none(),
    };
    if !hand_empty || game.spectator() {
        return;
    }
    if !crate::pluto_sleeves_takeover_access::active() {
        return;
    }
    let Some(eye) = game.eye_position() else {
        return;
    };
    let post = crate::pluto_sleeves_takeover_access::PostUse {
        uuid: sunrise_client_lib::lan::playerdata::ints_uuid(game.acting_player_uuid()),
        name: game.acting_player_name().to_owned(),
        dimension: game.dim.id().to_owned(),
        eye,
        look: look_direction(game.yaw, game.pitch),
        crouching: game.keys.held(Mapping::Sneak),
        creative: game.creative,
        // The gate above already established the interaction was not consumed.
        consumed: false,
    };
    let held = crate::game::pluto_item_cooldown::hold(game);
    crate::pluto_sleeves_takeover_access::on_post_use(game, &post);
    held.settle(game);
}

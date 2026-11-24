// After the hand's use returns: the chestplate takeover (Tinkers' InteractionHandler, at its
// LOWEST priority, after the game handled the click) runs the worn sleeves module when the
// use did not consume anything. The game reports the use's outcome; the pure side owns the
// piece, the selection and the throw.
use super::*;

use horizon_client::mod_api::Host;

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

/// Runs the post-use takeover for a use the game did not consume: nothing happens when the
/// game consumed the use (or the use failed), when the hand in use still holds something,
/// when the player is a spectator, when the eye cannot be read, or when no mod listens. The
/// cooldown surface reads the game this call runs in, held for the provider call only.
pub(crate) fn after_use(game: &mut Game<'_>, value: Result<bool>) {
    if !matches!(value, Ok(false)) {
        return;
    }
    let hand_empty = match game.hand {
        Hand::Main => game.held().is_none(),
        Hand::Off => game.offhand.is_none(),
    };
    if !hand_empty || game.spectator() {
        return;
    }
    if !crate::entry::pluto_sleeves_takeover_access::active() {
        return;
    }
    let Some(eye) = game.eye_position() else {
        return;
    };
    let post = crate::entry::pluto_sleeves_takeover_access::PostUse {
        uuid: horizon_client::lan::playerdata::ints_uuid(game.acting_player_uuid()),
        name: game.acting_player_name().to_owned(),
        dimension: game.dim.id().to_owned(),
        eye,
        look: look_direction(game.yaw, game.pitch),
        crouching: game.keys.held(Mapping::Sneak),
        creative: game.creative,
        // The gate above already established the use was not consumed.
        consumed: false,
    };
    let _held = crate::game::pluto_item_cooldown::hold(game);
    crate::entry::pluto_sleeves_takeover_access::on_post_use(game, &post);
}

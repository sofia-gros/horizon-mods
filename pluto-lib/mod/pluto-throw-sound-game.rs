// The game's half of the throw-sound reach (pluto-throw-sound-access): the MODIFY_VARIABLE reach on
// `Game::throw_item` asks here for the sound a thrown type throws with, and the game's own snowball throw
// stays when the type carries no throw sound (or nothing listens). The volume and pitch the game already
// plays are Java's (0.5, `0.4 / (random * 0.4 + 0.8)`); the event id is swapped, and for a type with a
// registered throw sound the play position is too (throw_position below) - Java plays a gadget throw
// at the thrower's feet, where the game plays its own throw at the eye.
// The reach binds the thrown stack - a binding may name only a target parameter, never the local `kind` -
// so the type is recovered with the host's own `thrown_by_item`, the same lookup `throw_item` built the
// kind from: at the `sound` store it names a type exactly when the kind is that type's `Thrown`, and a
// registered type never shadows a vanilla throwable (the API contract forbids a `minecraft:` type id).
use crate::entry::pluto_throw_sound_access as access;

pub(crate) fn throw_sound(value: &'static str, stack: &crate::items::ItemStack) -> &'static str {
    match crate::projectiles::thrown_by_item(&*stack.item) {
        Some(t) => access::throw_sound(t.id).unwrap_or(value),
        None => value,
    }
}

/// The position a throw plays at: the thrower's feet (the player motion's position) for a type
/// with a registered throw sound - Java's `playSound(null, player.getX(), player.getY(),
/// player.getZ(), ...)` of the gadget items - and the game's own position (the eye) otherwise,
/// so a vanilla throwable and a soundless type keep the game's play untouched.
pub(crate) fn throw_position(value: Option<[f64; 3]>, stack: &crate::items::ItemStack, game: &crate::game::Game<'_>) -> Option<[f64; 3]> {
    match crate::projectiles::thrown_by_item(&*stack.item) {
        Some(t) if access::throw_sound(t.id).is_some() => match game.player.as_ref() {
            Some(player) => Some([player.position.x, player.position.y, player.position.z]),
            None => value,
        },
        _ => value,
    }
}

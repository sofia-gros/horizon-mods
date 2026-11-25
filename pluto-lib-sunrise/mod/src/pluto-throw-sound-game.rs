// The game's half of the throw-sound reach (pluto-throw-sound-access): the MODIFY_VARIABLE reach on
// `Game::throw_item` asks here for the sound a thrown type throws with, and the game's own snowball throw
// stays when the type carries no throw sound (or nothing listens). The volume and pitch the game already
// plays are Java's (0.5, `0.4 / (random * 0.4 + 0.8)`); only the event id is swapped.
// The reach binds the thrown stack - a binding may name only a target parameter, never the local `kind` -
// so the type is recovered with the host's own `thrown_by_item`, the same lookup `throw_item` built the
// kind from: at the `sound` store it names a type exactly when the kind is that type's `Thrown`, and a
// registered type never shadows a vanilla throwable (the API contract forbids a `minecraft:` type id).
use crate::pluto_throw_sound_access as access;

pub(crate) fn throw_sound(value: &'static str, stack: &crate::items::ItemStack) -> &'static str {
    match crate::projectiles::thrown_by_item(&*stack.item) {
        Some(t) => access::throw_sound(t.id).unwrap_or(value),
        None => value,
    }
}

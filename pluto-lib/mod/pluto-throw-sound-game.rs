// The game's half of the throw-sound reach (pluto-throw-sound-access): the MODIFY_VARIABLE reach on
// `Game::throw_item` asks here for the sound a thrown type throws with, and the game's own snowball throw
// stays when the type carries no throw sound (or nothing listens). The volume and pitch the game already
// plays are Java's (0.5, `0.4 / (random * 0.4 + 0.8)`); only the event id is swapped.
use crate::entry::pluto_throw_sound_access as access;

pub(crate) fn throw_sound(value: &'static str, kind: &crate::projectiles::ProjectileKind) -> &'static str {
    match kind {
        crate::projectiles::ProjectileKind::Thrown(t) => access::throw_sound(t.id).unwrap_or(value),
        _ => value,
    }
}

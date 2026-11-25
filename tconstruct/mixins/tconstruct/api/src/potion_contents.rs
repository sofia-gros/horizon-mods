//! A potion's contents (PotionUtils.getPotion): the effects a potion ID carries.
//!
//! PlutoLib's potion reach reads the game's own potion registry (`pluto_lib::Potions`),
//! so the effects are the game's, and a potion the game does not know carries none —
//! the original's tipped module does nothing for an invalid ID the same way. The tipped
//! application in `flight_hook` runs against these.
use crate::fluid_effects::PotionEffect;
use pluto_lib::{Identifier, Potions};

/// The effects of the potion `potion` (empty when the game has no potion under that ID).
pub(crate) fn effects(potion: &Identifier) -> Vec<PotionEffect> {
    Potions::effects(potion)
        .unwrap_or_default()
        .into_iter()
        .map(|fact| PotionEffect { effect: fact.effect, duration: fact.duration, amplifier: fact.amplifier, instant: fact.instant })
        .collect()
}

/// TippedModule's divisor for a tipped entry of the given int level:
/// `1 << max(4 - level, 0)`.
pub(crate) fn divisor(level: i32) -> i32 {
    1 << (4 - level).max(0)
}

/// The duration a tipped hit applies: the potion's duration over the divisor, at
/// least one tick.
pub(crate) fn scaled_duration(duration: i32, level: i32) -> i32 {
    (duration / divisor(level)).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_divisor_halves_every_level_past_the_first_four() {
        assert_eq!(divisor(1), 8);
        assert_eq!(divisor(4), 1);
        assert_eq!(divisor(7), 1);
        assert_eq!(scaled_duration(600, 1), 75);
        assert_eq!(scaled_duration(3, 1), 1, "a short effect still lands for a tick");
    }
}

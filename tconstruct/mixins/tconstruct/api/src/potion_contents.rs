//! A potion's contents (PotionUtils.getPotion): the effects a potion ID carries.
//!
//! No PlutoLib reach hands out a potion's effects — the host keeps potions in its own
//! code and its mod API has no potion call, the same gap `fluid_world` records for the
//! potion fluid effects — so this reads empty today. The tipped application in
//! `flight_hook` runs against it exactly as the fluid effects run against theirs: the
//! wiring is complete, and the contents arrive when a potion reach does.
use crate::fluid_effects::PotionEffect;
use pluto_lib::Identifier;

/// The effects of the potion `potion` (empty while no reach reports potion contents).
pub(crate) fn effects(_potion: &Identifier) -> Vec<PotionEffect> {
    Vec::new()
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

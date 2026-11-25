//! Potions: the effects a potion carries (Minecraft: `Potions`, `PotionUtils.getPotion`).
//!
//! The game keeps the potion registry in its own code: which effects each potion holds, at what
//! duration and amplifier. PlutoLib reads that registry from the game — the game-facing half
//! installs the lookup when it starts — so a mod asks here instead of copying the vanilla table,
//! and a potion another mod registered with the game is answered like a vanilla one. Without the
//! game (unit tests, tools) a lookup finds nothing, as [`Potions::effects`] says.

use std::sync::OnceLock;

use super::base::Identifier;

/// One effect of a potion, as the game's registry lists it: the effect, how long it lasts and at
/// what amplifier, and whether the effect is instantaneous (it applies at once instead of lasting).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PotionEffectFact {
    /// The effect's id (`minecraft:poison`).
    pub effect: Identifier,
    /// The duration in ticks (an instantaneous effect's stored duration is 1).
    pub duration: i32,
    /// The amplifier: the effect's level less one.
    pub amplifier: i32,
    /// Whether the effect is instantaneous.
    pub instant: bool,
}

/// What the game says about potions (set once by the game-facing half).
pub(crate) struct HostPotions {
    /// A potion's effects as (effect id, duration ticks, amplifier, instantaneous), in registry
    /// order; None when the game has no potion under the id.
    pub effects: fn(&str) -> Option<Vec<(String, i32, i32, bool)>>,
}

static HOST: OnceLock<HostPotions> = OnceLock::new();

pub(crate) fn set_host(host: HostPotions) {
    let _ = HOST.set(host);
}

/// Looking potions up in the game's registry.
pub struct Potions;

impl Potions {
    /// The effects of the potion `potion`, in the order the game lists them.
    ///
    /// None when the game has no potion under that id — an invalid id is not an error, it simply
    /// carries no effects (the original's tipped module notes the same). A known potion with no
    /// effects (water, mundane) answers Some with an empty list. Without the game, None.
    pub fn effects(potion: &Identifier) -> Option<Vec<PotionEffectFact>> {
        let host = HOST.get()?;
        let rows = (host.effects)(&potion.to_string())?;
        Some(facts(rows))
    }

    /// The effects a potion's contents apply, as a stack carries them (`PotionContents.getEffects`):
    /// the potion's own effects followed by the contents' custom effects, in that order. An unknown
    /// potion contributes no effects of its own, so the custom effects are the whole list.
    pub fn contents(potion: &Identifier, custom_effects: &[PotionEffectFact]) -> Vec<PotionEffectFact> {
        let mut all = Self::effects(potion).unwrap_or_default();
        all.extend(custom_effects.iter().cloned());
        all
    }
}

/// The host's rows as facts; a row whose effect id does not parse is dropped, never invented.
fn facts(rows: Vec<(String, i32, i32, bool)>) -> Vec<PotionEffectFact> {
    rows.into_iter()
        .filter_map(|(effect, duration, amplifier, instant)| {
            Identifier::parse(&effect).ok().map(|effect| PotionEffectFact { effect, duration, amplifier, instant })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    #[test]
    fn host_rows_become_facts_in_order_and_an_unparsable_row_is_dropped() {
        let rows = vec![
            ("minecraft:poison".to_owned(), 432, 1, false),
            ("minecraft:instant_health".to_owned(), 1, 0, true),
            ("not an id".to_owned(), 10, 0, false),
        ];
        assert_eq!(
            facts(rows),
            [
                PotionEffectFact { effect: id("minecraft:poison"), duration: 432, amplifier: 1, instant: false },
                PotionEffectFact { effect: id("minecraft:instant_health"), duration: 1, amplifier: 0, instant: true },
            ]
        );
    }

    #[test]
    fn without_a_game_a_lookup_finds_nothing_and_contents_are_the_custom_effects() {
        // The pure test crate never installs a host, so this also pins the fallback every
        // host-backed lookup shares: no game, no registry, no effects.
        assert_eq!(Potions::effects(&id("minecraft:strong_poison")), None);
        let custom = PotionEffectFact { effect: id("minecraft:poison"), duration: 200, amplifier: 0, instant: false };
        assert_eq!(Potions::contents(&id("minecraft:water"), &[custom.clone()]), [custom]);
    }
}

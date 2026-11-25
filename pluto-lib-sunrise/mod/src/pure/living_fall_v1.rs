//! A living entity's fall, before the fall damage is applied (the host's
//! `LivingFallEvent`, API 357, `native-hooks.json`): the distance and the damage
//! multiplier may be changed, and answering `false` cancels the fall handling
//! (the block's own `fallOn` still runs with zero damage, exactly as the native
//! living-fall cancellation branch shapes it). The host fires it for the local
//! player (`landing.rs`, the player's `cause_fall_damage`) and for every mob
//! (`mobs/living.rs`, `cause_fall_damage_at`, after the per-type cause-fall
//! sink declines), so one listener covers both landings; a mob with a
//! per-type fall behaviour never reaches it. The event carries no velocity:
//! the host's fall call does not hand the motion over, so a callback that
//! needs it reads the motion it kept from the entity's last tick instead.

use std::cell::Cell;

use super::base::Event;

/// A living entity is about to handle a fall: its entity id, its type (such
/// as `minecraft:player`), the fall distance in blocks, and the damage
/// multiplier (1 by default). A callback changes the distance or the
/// multiplier by setting the cells; answering `false` cancels the fall.
#[derive(Debug)]
pub struct LivingFall {
    /// The falling entity's id.
    pub entity: u32,
    /// The falling entity's type.
    pub kind: String,
    /// The fall distance, in blocks: the entity's own fall distance, as the
    /// host's fall call receives it.
    pub distance: Cell<f64>,
    /// The damage multiplier.
    pub multiplier: Cell<f32>,
}

static FALL: Event<LivingFall, bool> = Event::allow_all();

/// Events about a living entity's fall.
pub struct LivingFallEvents;

impl LivingFallEvents {
    /// A living entity is about to handle a fall, before the fall damage is
    /// applied. Answer `false` to cancel the fall handling (the block's own
    /// `fallOn` still runs with zero damage); one `false` is enough. Heard
    /// for the local player and for mobs, on both sides.
    pub const FALL: &'static Event<LivingFall, bool> = &FALL;
}

/// Whether anyone listens to falls, so the host only builds the payload when someone does.
pub(crate) fn fall_heard() -> bool {
    !FALL.is_empty()
}

/// Tells the fall to the callbacks; whether the fall stands (false when a callback cancelled it).
pub(crate) fn fall(fall: &LivingFall) -> bool {
    FALL.invoke(fall)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fall_listener_changes_the_distance_and_multiplier_and_cancels_only_its_entity() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        LivingFallEvents::FALL.register(move |fall| {
            if fall.entity == 710_001 {
                log.lock().unwrap().push(fall.kind.clone());
                fall.distance.set(0.0);
                fall.multiplier.set(0.0);
                return false;
            }
            true
        });
        let cancelled = LivingFall {
            entity: 710_001,
            kind: "minecraft:player".to_owned(),
            distance: Cell::new(9.0),
            multiplier: Cell::new(1.0),
        };
        assert!(!fall(&cancelled));
        assert_eq!(cancelled.distance.get(), 0.0);
        assert_eq!(cancelled.multiplier.get(), 0.0);
        assert_eq!(*seen.lock().unwrap(), vec!["minecraft:player".to_string()]);
        let standing = LivingFall {
            entity: 710_002,
            kind: "minecraft:zombie".to_owned(),
            distance: Cell::new(9.0),
            multiplier: Cell::new(1.0),
        };
        assert!(fall(&standing));
        assert_eq!(standing.distance.get(), 9.0);
        assert_eq!(standing.multiplier.get(), 1.0);
    }
}

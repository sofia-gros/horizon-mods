//! What a dispenser does with an item a mod claims (Minecraft: `DispenserBlock.registerBehavior`).
//!
//! The game already asks its mods before it dispenses: the host's dispense hook (its native hook 67) lets
//! the first mod that answers replace the vanilla ejection, name what stays in the slot, and choose the
//! level event played with it. PlutoLib carries that answer for the behaviours mods register here; an item
//! nobody claims keeps the game's own dispensing, untouched.
//!
//! The one behaviour this surface starts with is the shot: the original's
//! `ShootProjectileDispenserBehavior`, which spawns the item's own projectile entity at the dispenser's
//! mouth, aims it along the facing and consumes one item. The host's hook hands over no random source, so
//! the shot is the behaviour's mean shot: the inaccuracy spread is not applied (see [`dispense_shot`]).

use super::base::Identifier;
use super::block_api_v1::Direction;
use std::collections::BTreeMap;
use std::sync::RwLock;

/// What a dispenser does with one item.
#[derive(Clone, Debug, PartialEq)]
pub enum DispenseBehavior {
    /// Fires the item as a projectile of the entity type `entity` (the item's own thrown form), at `power`
    /// with `inaccuracy` (the original's `ShootProjectileDispenserBehavior` numbers: its Tinkers throwables
    /// shoot at power 1.1, inaccuracy 6.0). One item is consumed per shot.
    Shoot {
        /// The projectile entity type to spawn, such as `tconstruct:glow_ball`.
        entity: Identifier,
        /// The shot's speed (the original's `getPower`).
        power: f32,
        /// The shot's spread (the original's `getInaccuracy`); recorded for parity, not applied — the
        /// dispense callback is handed no random source, so the shot flies true (see [`dispense_shot`]).
        inaccuracy: f32,
    },
}

static BEHAVIORS: RwLock<BTreeMap<String, DispenseBehavior>> = RwLock::new(BTreeMap::new());

/// The registry of dispenser behaviours, keyed by the item dispensed.
pub struct DispenseRegistry;

impl DispenseRegistry {
    /// Claims `item` for `behavior`: a dispenser that would eject it runs the behaviour instead. Registering
    /// the same item again replaces its behaviour.
    pub fn register(item: Identifier, behavior: DispenseBehavior) {
        if let Ok(mut behaviors) = BEHAVIORS.write() {
            behaviors.insert(item.to_string(), behavior);
        }
    }

    /// The behaviour claimed for `item`, if any.
    pub fn get(item: &str) -> Option<DispenseBehavior> {
        BEHAVIORS.read().ok()?.get(item).cloned()
    }
}

/// The shot a [`DispenseBehavior::Shoot`] makes: where the projectile spawns and the motion it starts with.
///
/// The spawn point is the dispenser's dispense position (the original's `DispenserBlock.getDispensePosition`:
/// the block's centre stepped 0.7 along the facing — 0.5 from the block corner plus the step). The motion is
/// the original's `shoot` with no spread: the facing stepped up by 0.1, normalised, scaled by `power`. The
/// spread the inaccuracy would add (`direction + gaussian * 0.0075 * inaccuracy`, then normalised) needs a
/// random source the host's dispense hook does not hand over, so the shot is the mean shot; a mod that needs
/// the spread must accept that this surface does not roll it.
pub fn dispense_shot(pos: [i32; 3], facing: Direction, power: f32) -> ([f64; 3], [f64; 3]) {
    let step = facing.offset();
    let origin = [
        f64::from(pos[0]) + 0.5 + 0.7 * f64::from(step[0]),
        f64::from(pos[1]) + 0.5 + 0.7 * f64::from(step[1]),
        f64::from(pos[2]) + 0.5 + 0.7 * f64::from(step[2]),
    ];
    let direction = [f64::from(step[0]), f64::from(step[1]) + 0.1, f64::from(step[2])];
    let length = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    let scale = if length > 0.0 { f64::from(power) / length } else { 0.0 };
    (origin, [direction[0] * scale, direction[1] * scale, direction[2] * scale])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    #[test]
    fn a_registered_item_answers_and_an_unregistered_one_does_not() {
        let behavior = DispenseBehavior::Shoot { entity: id("example:glow_ball"), power: 1.1, inaccuracy: 6.0 };
        DispenseRegistry::register(id("example:glow_ball"), behavior.clone());
        assert_eq!(DispenseRegistry::get("example:glow_ball"), Some(behavior));
        assert_eq!(DispenseRegistry::get("example:unclaimed"), None);
        // registering again replaces
        let faster = DispenseBehavior::Shoot { entity: id("example:glow_ball"), power: 2.0, inaccuracy: 6.0 };
        DispenseRegistry::register(id("example:glow_ball"), faster.clone());
        assert_eq!(DispenseRegistry::get("example:glow_ball"), Some(faster));
    }

    fn close(actual: [f64; 3], expected: [f64; 3]) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() < 1e-9, "axis {axis}: {} != {}", actual[axis], expected[axis]);
        }
    }

    #[test]
    fn a_shot_leaves_the_mouth_aimed_along_the_facing_and_slightly_up() {
        // east from the dispenser at (10, 64, -3): centre (10.5, 64.5, -2.5) stepped 0.7 east
        let (origin, motion) = dispense_shot([10, 64, -3], Direction::East, 1.1);
        close(origin, [11.2, 64.5, -2.5]);
        // the motion is (1, 0.1, 0) normalised, scaled by the power: its length is the power
        let length = (motion[0] * motion[0] + motion[1] * motion[1] + motion[2] * motion[2]).sqrt();
        assert!((length - f64::from(1.1f32)).abs() < 1e-9, "the shot flies at its power: {length}");
        assert!(motion[0] > 0.0 && motion[1] > 0.0 && motion[2] == 0.0, "east and a touch up: {motion:?}");
        assert!((motion[1] / motion[0] - 0.1).abs() < 1e-9, "the 0.1 rise is kept: {motion:?}");
    }

    #[test]
    fn a_shot_up_leaves_through_the_top() {
        let (origin, motion) = dispense_shot([0, 70, 0], Direction::Up, 2.0);
        close(origin, [0.5, 71.2, 0.5]);
        close(motion, [0.0, 2.0, 0.0]);
    }
}

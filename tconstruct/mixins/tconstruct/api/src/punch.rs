//! PunchModule and ReversePunchModule, from SlimeKnights (MIT; see LICENSE): the knockback a launcher's
//! projectiles carry, and the pull that replaces it on a hit. Punch runs as the projectile is shot and sets
//! the arrow's own knockback (a knockback-aware projectile's is added to instead); reverse punch runs when
//! the projectile hits an entity and pushes the target back along the shot's line, against the way the
//! arrow flew. PlutoLib's shot event cannot edit the arrow it reports, and its impact event carries
//! neither the projectile's modifiers nor its motion, so both consumers are worked out here over the data
//! the original's hooks receive; nothing calls them in game yet.
use crate::modifier_entries::Entry;
use serde_json::{Map, Value};

/// A LevelingValue read straight from a module's own object (LevelingValue.directField).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Leveling { flat: f32, each: f32 }

impl Leveling {
    fn direct(fields: &Map<String, Value>) -> Result<Self, String> {
        let number = |key: &str| -> Result<f32, String> {
            match fields.get(key).filter(|value| !value.is_null()) {
                None => Ok(0.0),
                Some(value) => value.as_f64().map(|value| value as f32).ok_or_else(|| format!("{key} must be a number")),
            }
        };
        Ok(Self { flat: number("flat")?, each: number("each_level")? })
    }
    /// LevelingValue.compute: flat plus each_level times the level.
    fn compute(self, level: f32) -> f32 { self.flat + self.each * level }
}

/// What one shot does to the projectile's knockback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Shot {
    /// AbstractArrow.setKnockback: the arrow's knockback becomes this (an int in the original).
    Set(i32),
    /// ProjectileWithKnockback.addKnockback: this much is added to the projectile's knockback.
    Add(f32),
}

/// PunchModule: a leveling amount of knockback for the tool's projectiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Punch { amount: Leveling }

impl Punch {
    pub fn decode(fields: &Map<String, Value>) -> Result<Self, String> { Ok(Self { amount: Leveling::direct(fields)? }) }

    /// ProjectileLaunchModifierHook.onProjectileShoot for one projectile: what happens to its knockback,
    /// when the module's condition held and the amount for the entry's level is positive. `arrow` is
    /// whether the projectile is an arrow (its knockback is a field that is set, cut to an int).
    pub fn shoot(&self, entry: &Entry, condition: bool, arrow: bool) -> Option<Shot> {
        if !condition { return None; }
        let amount = self.amount.compute(entry.effective_level());
        if amount <= 0.0 { return None; }
        Some(if arrow { Shot::Set(amount as i32) } else { Shot::Add(amount) })
    }
}

/// ReversePunchModule: a projectile hit that pulls its target back toward the shooter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ReversePunch { amount: Leveling }

impl ReversePunch {
    pub fn decode(fields: &Map<String, Value>) -> Result<Self, String> { Ok(Self { amount: Leveling::direct(fields)? }) }

    /// ProjectileHitModifierHook.onProjectileHitEntity: the push the target takes, as a motion to add
    /// ([x, y, z]), or None when there is nothing to push with. `motion` is the projectile's horizontal
    /// motion (x, z); `owned` is whether the projectile has an owner or is an arrow (only then is the base
    /// knockback a living target takes from a hit reversed out, as +0.8); `resistance` is the target's
    /// knockback resistance attribute, which scales what is left.
    pub fn push(&self, entry: &Entry, owned: bool, resistance: f32, motion: [f64; 2]) -> Option<[f64; 3]> {
        let mut amount = f64::from(self.amount.compute(entry.effective_level()));
        if owned { amount += 0.8; }
        amount *= f64::from((1.0 - resistance).max(0.0));
        // Vec3.multiply(1, 0, 1).normalize().scale(amount): a (nearly) still projectile has no line to
        // push along (normalize of a vector shorter than 1e-4 is the zero vector in the original).
        let length = (motion[0] * motion[0] + motion[1] * motion[1]).sqrt();
        if length < 1.0e-4 { return None; }
        let (x, z) = (motion[0] / length * amount, motion[1] / length * amount);
        // The original pushes only when the scaled motion is not the zero vector (an amount of 0).
        if x * x + z * z <= 0.0 { return None; }
        Some([-x, 0.1, -z])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::Identifier;
    use serde_json::json;

    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:punch").unwrap(), level) }

    #[test]
    fn punch_sets_an_arrows_knockback_and_adds_to_other_projectiles() {
        // The shipped punch module: 1 per level.
        let punch = Punch::decode(json!({"each_level": 1.0}).as_object().unwrap()).unwrap();
        assert_eq!(punch.shoot(&entry(2), true, true), Some(Shot::Set(2)));
        assert_eq!(punch.shoot(&entry(2), true, false), Some(Shot::Add(2.0)));
        // A fractional amount is cut to an int for an arrow, as the (int) cast does.
        let half = Punch::decode(json!({"each_level": 0.5}).as_object().unwrap()).unwrap();
        assert_eq!(half.shoot(&entry(3), true, true), Some(Shot::Set(1)));
        assert_eq!(half.shoot(&entry(3), true, false), Some(Shot::Add(1.5)));
    }

    #[test]
    fn punch_does_nothing_without_a_positive_amount_or_its_condition() {
        let punch = Punch::decode(json!({"each_level": 1.0}).as_object().unwrap()).unwrap();
        assert_eq!(punch.shoot(&entry(2), false, true), None);
        let none = Punch::decode(json!({}).as_object().unwrap()).unwrap();
        assert_eq!(none.shoot(&entry(2), true, true), None, "no flat and no each_level is an amount of 0");
    }

    #[test]
    fn reverse_punch_pulls_the_target_back_along_the_shots_line() {
        // The shipped drawback module: 0.6 per level.
        let drawback = ReversePunch::decode(json!({"each_level": 0.6}).as_object().unwrap()).unwrap();
        // Level 1 with an owner: (0.6 + 0.8) * (1 - 0.25) = 1.05 along (3, 4) normalized, reversed.
        let push = drawback.push(&entry(1), true, 0.25, [3.0, 4.0]).unwrap();
        assert!((push[0] - -0.63).abs() < 1e-6 && (push[1] - 0.1).abs() < 1e-6 && (push[2] - -0.84).abs() < 1e-6, "{push:?}");
        // Without an owner or an arrow behind it, the base knockback stays: 0.6 * 0.75 = 0.45.
        let push = drawback.push(&entry(1), false, 0.25, [3.0, 4.0]).unwrap();
        assert!((push[0] - -0.27).abs() < 1e-6 && (push[2] - -0.36).abs() < 1e-6, "{push:?}");
        // Full knockback resistance, or a still projectile, pushes nothing.
        assert_eq!(drawback.push(&entry(1), true, 1.0, [3.0, 4.0]), None);
        assert_eq!(drawback.push(&entry(1), true, 0.0, [0.0, 0.0]), None);
    }
}

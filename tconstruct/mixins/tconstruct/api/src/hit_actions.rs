//! FieryAttackModule, FreezingAttackModule, FieryCounterModule, FreezingCounterModule and KnockbackCounterModule, from SlimeKnights (MIT;
//! see LICENSE): hit effects that act on an entity instead of putting a mob effect on it. A melee hit sets the mob on fire or freezes it;
//! armor that counters does the same to the mob that hit its wearer, or knocks it back. Applying the action is PlutoLib's (it guards fire
//! immunity, freeze immunity and players); this only decides which action lands and how strong. Not ported: the armor's durability cost
//! (`durability_usage`, the hit reaches the mods before the wear is worked out), the doubled level of a blocking shield (only worn armor is
//! reported), the projectile half of the attack modules (no projectile hit event), the vanilla fire hack that makes an instant kill drop
//! cooked loot (the fire lands before the damage anyway), and the slot-in-charge rule of the knockback counter (see `strongest_knockback`).
use crate::{melee_damage::TargetCondition, modifier_entries::Entry, thorns::Leveling};
use pluto_lib::{ActionTarget, EntityAction, Identifier, KnockbackDirection};
use serde_json::{Map, Value};

/// What a module does to the entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effect { Fire, Freeze, Knockback }

/// Math.round for a positive amount of seconds.
fn seconds(value: f32) -> Option<i32> { let seconds = value.round() as i32; (seconds > 0).then_some(seconds) }

/// The ticks frozen for an amount of seconds: `(int)(value * 40)`.
fn freeze_ticks(value: f32) -> Option<i32> { let ticks = (value * 40.0) as i32; (ticks > 0).then_some(ticks) }

/// FieryAttackModule and FreezingAttackModule: a melee hit that sets the target on fire or freezes it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Attack { effect: Effect, time: Leveling }

impl Attack {
    /// `effect` is `Fire` or `Freeze`.
    pub(crate) fn decode(effect: Effect, fields: &Map<String, Value>) -> Result<Self, String> {
        if !fields.get("seconds").is_some_and(Value::is_object) { return Err("seconds must be an object".into()); }
        Ok(Self { effect, time: Leveling::decode(fields, "seconds", Leveling::ZERO)? })
    }

    /// afterMeleeHit: the action to do to the mob that was hit, when the modifier's condition holds. The hit is heard before its damage,
    /// so the action lands before it.
    pub(crate) fn action(&self, entry: &Entry, condition: bool) -> Option<EntityAction> {
        if !condition { return None; }
        let value = self.time.compute(entry.effective_level());
        match self.effect {
            Effect::Fire => seconds(value).map(|seconds| EntityAction::SetFire { target: ActionTarget::Victim, seconds }),
            _ => freeze_ticks(value).map(|ticks| EntityAction::Freeze { target: ActionTarget::Victim, ticks }),
        }
    }
}

/// FieryCounterModule, FreezingCounterModule and KnockbackCounterModule (CounterModule): armor that, with a chance that grows with the
/// level, sets the mob that hit its wearer on fire, freezes it or knocks it back. The value is a flat part plus a random share of the
/// random part, both scaled by the level.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Counter { effect: Effect, chance: Leveling, constant: Leveling, random: Leveling, attacker: TargetCondition }

impl Counter {
    /// `None` when a condition names state this port does not hold (a condition on the wearer, or on the weather or water).
    pub(crate) fn decode(effect: Effect, fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        if fields.get("defender").is_some_and(|value| !value.is_null()) { return Ok(None); }
        let Some(attacker) = TargetCondition::decode_for("attacker", fields.get("attacker"))? else { return Ok(None); };
        if !fields.get("chance").is_some_and(Value::is_object) { return Err("chance must be an object".into()); }
        let (constant, random) = if effect == Effect::Knockback { ("flat_knockback", "random_knockback") } else { ("constant_seconds", "random_seconds") };
        Ok(Some(Self {
            effect,
            chance: Leveling::decode(fields, "chance", Leveling::ZERO)?,
            constant: Leveling::decode(fields, constant, Leveling::ZERO)?,
            random: Leveling::decode(fields, random, Leveling::ZERO)?,
            attacker,
        }))
    }

    /// CounterModule.onAttacked: the value to apply, or None. `rolls` gives uniform values in [0, 1): the chance roll, then the random part's.
    pub(crate) fn value(&self, entry: &Entry, condition: bool, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<f32> {
        if !condition || !self.attacker.matches(in_tag) { return None; }
        let level = entry.effective_level();
        let chance = self.chance.compute(level);
        if !(chance > 0.0 && (chance >= 1.0 || rolls() < chance)) { return None; }
        let mut random = self.random.compute(level);
        if random > 0.0 { random *= rolls(); }
        let value = self.constant.compute(level) + random;
        (value > 0.0).then_some(value)
    }

    /// The action for a value, done to the mob that hit the wearer.
    pub(crate) fn action(&self, value: f32) -> Option<EntityAction> {
        let target = ActionTarget::Attacker;
        match self.effect {
            Effect::Fire => seconds(value).map(|seconds| EntityAction::SetFire { target, seconds }),
            Effect::Freeze => freeze_ticks(value).map(|ticks| EntityAction::Freeze { target, ticks }),
            // attacker.knockback(value, -sin(yaw), cos(yaw)): against the way the mob faces.
            Effect::Knockback => Some(EntityAction::Knockback { target, strength: f64::from(value), direction: KnockbackDirection::AgainstFacing }),
        }
    }
}

/// KnockbackCounterModule keeps the strongest push among the worn pieces (knockback does not add up like fire does): of all the knockback
/// actions only the one with the largest strength stays, in the place of the first. The original works the pieces out together under the
/// piece in charge's level; this port rolls each piece under its own, which agrees when the pieces carry the modifier at the same level.
pub(crate) fn strongest_knockback(actions: Vec<EntityAction>) -> Vec<EntityAction> {
    let strength = |action: &EntityAction| match action { EntityAction::Knockback { strength, .. } => Some(*strength), _ => None };
    let best = actions.iter().enumerate().filter_map(|(index, action)| strength(action).map(|value| (index, value)))
        .fold(None, |best: Option<(usize, f64)>, (index, value)| match best { Some((_, held)) if held >= value => best, _ => Some((index, value)) }).map(|(index, _)| index);
    actions.into_iter().enumerate().filter(|(index, action)| strength(action).is_none() || Some(*index) == best).map(|(_, action)| action).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:fiery").unwrap(), level) }
    fn any(_: &Identifier) -> bool { true }
    fn attack(effect: Effect, value: Value) -> Attack { Attack::decode(effect, value.as_object().unwrap()).unwrap() }
    fn counter(effect: Effect, value: Value) -> Option<Counter> { Counter::decode(effect, value.as_object().unwrap()).unwrap() }

    #[test]
    fn a_fiery_hit_burns_five_seconds_a_level_and_a_freezing_hit_adds_forty_ticks_a_second() {
        let fiery = attack(Effect::Fire, json!({"seconds":{"each_level":5.0}}));
        assert_eq!(fiery.action(&entry(2), true), Some(EntityAction::SetFire { target: ActionTarget::Victim, seconds: 10 }));
        assert_eq!(fiery.action(&entry(2), false), None, "the modifier condition holds first");
        let freezing = attack(Effect::Freeze, json!({"seconds":{"each_level":4.0,"flat":4.0}}));
        assert_eq!(freezing.action(&entry(1), true), Some(EntityAction::Freeze { target: ActionTarget::Victim, ticks: 320 }), "(4 + 4) seconds of 40 ticks");
        assert_eq!(freezing.action(&entry(3), true), Some(EntityAction::Freeze { target: ActionTarget::Victim, ticks: 640 }));
        assert!(Attack::decode(Effect::Fire, json!({}).as_object().unwrap()).is_err(), "seconds is required");
    }

    #[test]
    fn fiery_counter_armor_rolls_its_chance_then_burns_the_attacker_for_the_rounded_seconds() {
        let module = counter(Effect::Fire, json!({"chance":{"each_level":0.15},"constant_seconds":{"flat":1.0},"random_seconds":{"flat":6.0},"durability_usage":1})).unwrap();
        let value = module.value(&entry(2), true, &any, &mut || 0.25).unwrap();
        assert!((value - (1.0 + 6.0 * 0.25)).abs() < 1e-5, "{value}");
        assert_eq!(module.action(value), Some(EntityAction::SetFire { target: ActionTarget::Attacker, seconds: 3 }), "2.5 rounds to 3");
        assert!(module.value(&entry(2), true, &any, &mut || 0.31).is_none(), "30% at level 2: a roll over 0.3 misses");
        assert!(module.value(&entry(1), false, &any, &mut || 0.0).is_none(), "the modifier condition holds first");
        assert!(module.value(&entry(1), true, &|_: &Identifier| false, &mut || 0.0).is_some(), "no attacker condition accepts any mob");
    }

    #[test]
    fn freezing_counter_armor_freezes_forty_ticks_a_second_and_knockback_counter_pushes_against_the_facing() {
        let freezing = counter(Effect::Freeze, json!({"chance":{"each_level":0.15},"constant_seconds":{"flat":2.0},"random_seconds":{"flat":6.0},"durability_usage":1})).unwrap();
        let value = freezing.value(&entry(1), true, &any, &mut || 0.0).unwrap();
        assert_eq!(freezing.action(value), Some(EntityAction::Freeze { target: ActionTarget::Attacker, ticks: 80 }));
        let springy = counter(Effect::Knockback, json!({"chance":{"each_level":0.15},"durability_usage":1,"flat_knockback":{"flat":0.5},"random_knockback":{"flat":0.5}})).unwrap();
        let mut rolls = [0.0, 0.5].into_iter();
        let value = springy.value(&entry(1), true, &any, &mut || rolls.next().unwrap()).unwrap();
        assert!((value - 0.75).abs() < 1e-6);
        assert_eq!(springy.action(value), Some(EntityAction::Knockback { target: ActionTarget::Attacker, strength: 0.75, direction: KnockbackDirection::AgainstFacing }));
        assert!(springy.value(&entry(1), true, &any, &mut || 0.2).is_none(), "15% at level 1");
        let nothing = counter(Effect::Fire, json!({"chance":{"flat":1.0}})).unwrap();
        assert!(nothing.value(&entry(1), true, &any, &mut || 0.0).is_none(), "no value, no effect");
    }

    #[test]
    fn a_condition_this_port_cannot_hold_leaves_the_counter_unbound() {
        assert!(counter(Effect::Fire, json!({"chance":{"flat":1.0},"defender":"mantle:sprinting"})).is_none(), "a wearer condition stays unbound");
        assert!(counter(Effect::Fire, json!({"chance":{"flat":1.0},"attacker":{"type":"mantle:entity_type","entity_type":"minecraft:player"}})).is_none(), "a predicate over the entity type stays unbound");
        assert!(counter(Effect::Fire, json!({"chance":{"flat":1.0},"attacker":"mantle:on_fire"})).is_some());
        assert!(Counter::decode(Effect::Fire, json!({}).as_object().unwrap()).is_err(), "the chance is required");
    }

    #[test]
    fn only_the_strongest_knockback_of_the_worn_pieces_stays() {
        let push = |strength: f64| EntityAction::Knockback { target: ActionTarget::Attacker, strength, direction: KnockbackDirection::AgainstFacing };
        let fire = EntityAction::SetFire { target: ActionTarget::Attacker, seconds: 3 };
        assert_eq!(strongest_knockback(vec![push(0.6), fire.clone(), push(0.9), push(0.7)]), vec![fire.clone(), push(0.9)]);
        assert_eq!(strongest_knockback(vec![push(0.5), push(0.5)]), vec![push(0.5)], "equal pushes keep one");
        assert!(strongest_knockback(Vec::new()).is_empty());
    }
}

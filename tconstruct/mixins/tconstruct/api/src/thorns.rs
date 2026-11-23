//! ThornsModule, from SlimeKnights (MIT; see LICENSE): armor that hurts the mob that hit its wearer. Each piece rolls its chance (a share
//! that grows with the level), and the damage is a flat part plus a random share of the random part, both scaled by the level. The
//! attacker is filtered by entity type tags and what it was doing, including the water at its eyes and its feet, which the hurt report
//! fills from the mob's own fluid state; a condition on the wearer is state this port does not hold, so such a module stays unbound.
//! Three reductions stand: the armor's durability cost (`durability_usage`) is not charged (the hit reaches the mods before the wear is
//! worked out, and a worn piece is rewritten only through the item-wear event), the damage is not gated on the hit being direct
//! damage, and a blocking wearer does not double the level the chance and the damage scale with.
use crate::{melee_damage::TargetCondition, modifier_entries::Entry};
use pluto_lib::Identifier;
use serde_json::{Map, Value};

/// A LevelingValue: `flat + each_level * level`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Leveling { flat: f32, each: f32 }

impl Leveling {
    pub(crate) const ZERO: Self = Self { flat: 0.0, each: 0.0 };
    pub(crate) fn decode(fields: &Map<String, Value>, key: &str, default: Self) -> Result<Self, String> {
        match fields.get(key).filter(|value| !value.is_null()) {
            None => Ok(default),
            Some(Value::Object(value)) => {
                let float = |name: &str| value.get(name).and_then(Value::as_f64).map_or(0.0, |number| number as f32);
                Ok(Self { flat: float("flat"), each: float("each_level") })
            }
            Some(_) => Err(format!("{key} must be an object")),
        }
    }
    pub(crate) fn compute(self, level: f32) -> f32 { self.flat + self.each * level }
}

/// The damage a thorns module deals to the mob that hit the wearer.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Hurt { pub damage_type: Identifier, pub amount: f32 }

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Thorns { damage_type: Identifier, chance: Leveling, constant: Leveling, random: Leveling, attacker: TargetCondition }

impl Thorns {
    /// `None` when a condition names state this port does not hold.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let present = |key: &str| fields.get(key).is_some_and(|value| !value.is_null());
        if present("defender") { return Ok(None); }
        let Some(attacker) = TargetCondition::decode_for("attacker", fields.get("attacker"))? else { return Ok(None); };
        let damage_type = match fields.get("damage_type").and_then(Value::as_str) {
            None => Identifier::parse("minecraft:thorns").map_err(|e| e.to_string())?,
            Some(text) => Identifier::parse(text).map_err(|e| e.to_string())?,
        };
        Ok(Some(Self {
            damage_type,
            chance: Leveling::decode(fields, "chance", Leveling { flat: 0.0, each: 0.15 })?,
            constant: Leveling::decode(fields, "constant_damage", Leveling { flat: 0.0, each: 0.0 })?,
            random: Leveling::decode(fields, "random_damage", Leveling { flat: 0.0, each: 0.0 })?,
            attacker,
        }))
    }

    /// CounterModule.onAttacked and ThornsModule.applyEffect: the damage to deal, or None. `rolls` gives uniform values in [0, 1):
    /// the chance roll, then the random damage's.
    pub(crate) fn hurt(&self, entry: &Entry, condition: bool, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<Hurt> {
        if !condition || !self.attacker.matches(in_tag) { return None; }
        let level = entry.effective_level();
        let chance = self.chance.compute(level);
        if !(chance > 0.0 && (chance >= 1.0 || rolls() < chance)) { return None; }
        let mut random = self.random.compute(level);
        if random > 0.0 { random *= rolls(); }
        let amount = self.constant.compute(level) + random;
        (amount > 0.0).then(|| Hurt { damage_type: self.damage_type.clone(), amount })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn thorns(value: Value) -> Option<Thorns> { Thorns::decode(value.as_object().unwrap()).unwrap() }
    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:thorns").unwrap(), level) }
    fn any(_: &Identifier) -> bool { true }

    #[test]
    fn thorns_roll_their_chance_then_deal_a_flat_part_and_a_random_share() {
        let module = thorns(json!({"chance":{"each_level":0.15},"constant_damage":{"flat":1.0},"random_damage":{"flat":3.0},"damage_type":"minecraft:thorns"})).unwrap();
        let hurt = module.hurt(&entry(2), true, &any, &mut || 0.2).unwrap();
        assert_eq!(hurt.damage_type.to_string(), "minecraft:thorns");
        assert!((hurt.amount - (1.0 + 3.0 * 0.2)).abs() < 1e-5, "{}", hurt.amount);
        assert!(module.hurt(&entry(2), true, &any, &mut || 0.31).is_none(), "30% a level 2: a roll over 0.3 misses");
        assert!(module.hurt(&entry(2), false, &any, &mut || 0.0).is_none(), "the modifier condition holds first");
    }

    #[test]
    fn the_chance_defaults_to_fifteen_percent_a_level_and_no_damage_is_no_thorns() {
        let module = thorns(json!({"constant_damage":{"flat":2.0}})).unwrap();
        assert!(module.hurt(&entry(1), true, &any, &mut || 0.14).is_some());
        assert!(module.hurt(&entry(1), true, &any, &mut || 0.16).is_none());
        let none = thorns(json!({"chance":{"flat":1.0}})).unwrap();
        assert!(none.hurt(&entry(1), true, &any, &mut || 0.0).is_none(), "no damage parts, no thorns");
    }

    #[test]
    fn a_condition_this_port_cannot_hold_leaves_the_module_unbound() {
        assert!(thorns(json!({"attacker":{"type":"mantle:entity_type","entity_type":"minecraft:player"},"constant_damage":{"flat":2.0}})).is_none(), "a predicate over the entity type stays unbound");
        assert!(thorns(json!({"defender":"mantle:sprinting","constant_damage":{"flat":2.0}})).is_none(), "a wearer condition stays unbound");
        assert!(thorns(json!({"attacker":"mantle:on_fire","constant_damage":{"flat":2.0}})).is_some(), "a state the hit reports is bound");
    }

    #[test]
    fn the_shipped_shock_thorns_bind_and_answer_the_water_at_the_attacker() {
        // shock, first thorns: the attacker in water at its eyes or at its feet.
        let wet = thorns(json!({"type":"tconstruct:thorns","attacker":{"type":"mantle:or","predicates":["mantle:eyes_in_water","mantle:feet_in_water"]},
            "chance":{"each_level":0.2},"constant_damage":{"flat":2.0},"damage_type":"tconstruct:shock","durability_usage":1,
            "random_damage":{"flat":3.0},"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}})).unwrap();
        let state = |eyes_in_water: bool, in_water: bool| pluto_lib::EntityState { eyes_in_water, in_water, ..Default::default() };
        let answers = |state: &pluto_lib::EntityState| move |tag: &Identifier| crate::melee_damage::state_in_tag(tag, Some(state), None);
        let dry = state(false, false);
        assert!(wet.hurt(&entry(1), true, &answers(&dry), &mut || 0.0).is_none(), "a dry attacker is not shocked");
        let eyes = state(true, false);
        assert_eq!(wet.hurt(&entry(1), true, &answers(&eyes), &mut || 0.0), Some(Hurt { damage_type: Identifier::parse("tconstruct:shock").unwrap(), amount: 2.0 }), "the eyes in water are enough");
        let feet = state(false, true);
        assert!(wet.hurt(&entry(1), true, &answers(&feet), &mut || 0.0).is_some(), "the feet in water are enough");
        // shock, second thorns: rain on the attacker.
        let rained = thorns(json!({"type":"tconstruct:thorns","attacker":"mantle:raining_at",
            "chance":{"each_level":0.1},"constant_damage":{"flat":2.0},"damage_type":"tconstruct:shock","durability_usage":1,
            "random_damage":{"flat":3.0},"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}})).unwrap();
        let rain = pluto_lib::EntityState { rain_exposed: true, ..Default::default() };
        assert!(rained.hurt(&entry(1), true, &answers(&rain), &mut || 0.0).is_some(), "rain on the attacker is enough");
        assert!(rained.hurt(&entry(1), true, &answers(&dry), &mut || 0.0).is_none(), "no rain, no shock");
    }
}

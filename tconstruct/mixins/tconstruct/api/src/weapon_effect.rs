//! MobEffectModule.Weapon, from SlimeKnights (MIT; see LICENSE): a melee hit that may put a mob effect on the target. The level and
//! the duration are RandomLevelingValues (flat + per level + a random bonus for each level), the effect applies with a chance that
//! scales with the level, and the target and the holder are entity conditions (the entity type tags and what the entities were
//! doing, as for melee damage). Applying the effect is the caller's: this only decides which effect lands and how strong.
use crate::{melee_damage::TargetCondition, modifier_entries::Entry};
use pluto_lib::Identifier;
use serde_json::{Map, Value};

/// RandomLevelingValue: `flat + per_level * level`, plus a random share of `random` for each level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RandomLeveling { flat: f32, per_level: f32, random: f32 }

impl RandomLeveling {
    fn decode(fields: &Map<String, Value>, key: &str) -> Result<Self, String> {
        let value = fields.get(key).and_then(Value::as_object).ok_or_else(|| format!("{key} must be an object"))?;
        let float = |name: &str| match value.get(name).filter(|value| !value.is_null()) {
            None => Ok(0.0),
            Some(Value::Number(number)) => number.as_f64().map(|value| value as f32).ok_or_else(|| format!("invalid {key}.{name}")),
            Some(_) => Err(format!("{key}.{name} must be a number")),
        };
        Ok(Self { flat: float("flat")?, per_level: float("per_level")?, random: float("random")? })
    }
    /// RandomLevelingValue.computeValue; `roll` is a uniform value in [0, 1).
    pub(crate) fn compute(&self, level: f32, roll: f32) -> f32 {
        let mut value = self.flat + self.per_level * level;
        if self.random > 0.0 && level > 0.0 { value += roll * level * self.random; }
        value
    }
}

/// The effect a hit puts on its target: the effect id, the amplifier (the level less one) and the duration in ticks.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Grant { pub effect: Identifier, pub amplifier: i32, pub duration: i32 }

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WeaponEffect {
    effect: Identifier, level: RandomLeveling, time: RandomLeveling, target: TargetCondition, holder: TargetCondition,
    chance_flat: f32, chance_each: f32, before_melee: bool,
}

impl WeaponEffect {
    /// `None` when a condition names state this port does not hold.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let effect = Identifier::parse(fields.get("effect").and_then(Value::as_str).ok_or("effect missing")?).map_err(|e| e.to_string())?;
        let level = RandomLeveling::decode(fields, "level")?;
        let time = RandomLeveling::decode(fields, "time")?;
        let Some(target) = TargetCondition::decode_for("target", fields.get("target"))? else { return Ok(None); };
        let Some(holder) = TargetCondition::decode_for("attacker", fields.get("holder"))? else { return Ok(None); };
        // LevelingValue.ONE unless a chance is given: flat 0 + 1 per level is not the default, the default is a flat 1.
        let (chance_flat, chance_each) = match fields.get("chance").filter(|value| !value.is_null()) {
            None => (1.0, 0.0),
            Some(Value::Object(chance)) => {
                let float = |name: &str| chance.get(name).and_then(Value::as_f64).map_or(0.0, |value| value as f32);
                (float("flat"), float("each_level"))
            }
            Some(_) => return Err("chance must be an object".into()),
        };
        let before_melee = fields.get("apply_before_melee").and_then(Value::as_bool).unwrap_or(false);
        Ok(Some(Self { effect, level, time, target, holder, chance_flat, chance_each, before_melee }))
    }
    /// Whether the effect applies before the hit's damage lands instead of after.
    pub(crate) fn before_melee(&self) -> bool { self.before_melee }

    /// MobEffectModule.checkChance, WeaponCommon.onMonsterMeleeHit and ModifierMobEffect.applyEffect: the effect to put on the target,
    /// or None. `rolls` gives uniform values in [0, 1): the chance roll, the level's, then the duration's.
    pub(crate) fn grant(&self, entry: &Entry, condition: bool, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<Grant> {
        if !condition { return None; }
        let scaled = entry.effective_level();
        let chance = self.chance_flat + self.chance_each * scaled;
        if !(chance > 0.0 && (chance >= 1.0 || rolls() < chance)) { return None; }
        if !self.holder.matches(in_tag) || !self.target.matches(in_tag) { return None; }
        let amplifier = self.level.compute(scaled, rolls()).round() as i32 - 1;
        if amplifier < 0 { return None; }
        let duration = self.time.compute(scaled, rolls());
        (duration > 0.0).then(|| Grant { effect: self.effect.clone(), amplifier, duration: duration as i32 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn decode(value: Value) -> Option<WeaponEffect> { WeaponEffect::decode(value.as_object().unwrap()).unwrap() }
    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:venom").unwrap(), level) }
    fn all_tags(_: &Identifier) -> bool { true }
    fn no_tags(_: &Identifier) -> bool { false }

    #[test]
    fn the_level_and_the_duration_scale_and_roll_like_a_random_leveling_value() {
        let spectral = decode(json!({"effect":"minecraft:glowing","level":{"flat":1.0},"time":{"per_level":200.0}})).unwrap();
        assert_eq!(spectral.grant(&entry(2), true, &all_tags, &mut || 0.0), Some(Grant { effect: Identifier::parse("minecraft:glowing").unwrap(), amplifier: 0, duration: 400 }));
        let venom = decode(json!({"effect":"tconstruct:venom","level":{"flat":1.0},"time":{"flat":100.0,"random":100.0}})).unwrap();
        assert_eq!(venom.grant(&entry(1), true, &all_tags, &mut || 0.5).unwrap().duration, 150, "half of the random bonus for one level");
        assert_eq!(venom.grant(&entry(3), true, &all_tags, &mut || 0.5).unwrap().duration, 250, "the random bonus counts for each level");
        let decay = decode(json!({"effect":"minecraft:wither","level":{"per_level":1.0},"time":{"flat":120.0}})).unwrap();
        assert_eq!(decay.grant(&entry(3), true, &all_tags, &mut || 0.0).unwrap().amplifier, 2, "level 3 is amplifier 2");
    }

    #[test]
    fn the_chance_the_conditions_and_a_nonpositive_level_or_time_stop_the_effect() {
        let chancy = decode(json!({"effect":"minecraft:poison","level":{"flat":1.0},"time":{"flat":100.0},"chance":{"each_level":0.25}})).unwrap();
        assert!(chancy.grant(&entry(2), true, &all_tags, &mut || 0.49).is_some(), "a roll under 50% lands");
        assert!(chancy.grant(&entry(2), true, &all_tags, &mut || 0.51).is_none(), "a roll over 50% does not");
        assert!(chancy.grant(&entry(4), true, &all_tags, &mut || 0.99).is_some(), "100% needs no roll to succeed");
        assert!(chancy.grant(&entry(2), false, &all_tags, &mut || 0.0).is_none(), "the modifier condition holds first");
        let picky = decode(json!({"effect":"minecraft:slowness","level":{"flat":4.0},"time":{"flat":20.0},
            "target":{"type":"mantle:mob_type","mobs":"minecraft:arthropod"}})).unwrap();
        assert!(picky.grant(&entry(1), true, &all_tags, &mut || 0.0).is_some());
        assert!(picky.grant(&entry(1), true, &no_tags, &mut || 0.0).is_none(), "the target must be an arthropod");
        let none = decode(json!({"effect":"minecraft:poison","level":{"flat":0.0},"time":{"flat":100.0}})).unwrap();
        assert!(none.grant(&entry(1), true, &all_tags, &mut || 0.0).is_none(), "level 0 is no effect");
        let instant = decode(json!({"effect":"minecraft:poison","level":{"flat":1.0},"time":{"flat":0.0}})).unwrap();
        assert!(instant.grant(&entry(1), true, &all_tags, &mut || 0.0).is_none(), "no duration is no effect");
    }

    #[test]
    fn the_holder_and_target_conditions_ask_what_the_entities_were_doing() {
        let burning = decode(json!({"effect":"minecraft:poison","level":{"flat":1.0},"time":{"flat":100.0},"target":"mantle:on_fire","holder":"mantle:sprinting"})).unwrap();
        let state_only = |tag: &Identifier| tag.namespace() == crate::melee_damage::STATE_NAMESPACE;
        assert!(burning.grant(&entry(1), true, &state_only, &mut || 0.0).is_some());
        let target_only = |tag: &Identifier| tag.path().starts_with("target/");
        assert!(burning.grant(&entry(1), true, &target_only, &mut || 0.0).is_none(), "the holder must be sprinting");
        assert!(decode(json!({"effect":"minecraft:poison","level":{"flat":1.0},"time":{"flat":100.0},"target":{"type":"mantle:in_biome","biome":"minecraft:plains"}})).is_none(), "an unknown condition stays unbound");
    }
}

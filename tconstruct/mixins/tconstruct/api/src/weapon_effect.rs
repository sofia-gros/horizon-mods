//! MobEffectModule.Weapon, from SlimeKnights (MIT; see LICENSE): a melee hit that may put a mob effect on the target. The level and
//! the duration are RandomLevelingValues (flat + per level + a random bonus for each level), the effect applies with a chance that
//! scales with the level, and the target and the holder are entity conditions (the entity type tags and what the entities were
//! doing, as for melee damage). Applying the effect is the caller's: this only decides which effect lands and how strong.
//! MobEffectModule.ToolUsage is here too: the effect a tool puts on its own holder as it is used (a melee swing, a shot, shearing a mob).
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

/// Whom a counter module's effect goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Whom { Attacker, Wearer }

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WeaponEffect {
    effect: Identifier, level: RandomLeveling, time: RandomLeveling, target: TargetCondition, holder: TargetCondition,
    chance_flat: f32, chance_each: f32, before_melee: bool, whom: Whom,
    /// MobEffectModule.ArmorAttack's damage source and direct damage conditions (any source and any damage for the others).
    source: crate::protection::SourceCondition, direct: Option<bool>,
}

impl WeaponEffect {
    /// `None` when a condition names state this port does not hold.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> { Self::decode_as(fields, false) }

    /// MobEffectModule.ArmorCounter: an armor piece that puts the effect on the mob that hit its wearer. The chance defaults to 15% a level,
    /// `target` asks about the attacking mob, and a `damage_source` condition is decoded like an armor attack's and checked against the
    /// hurt report's damage source when the counter lands. The module stays unbound when it conditions the wearer (`holder`), limits the
    /// direct damage to anything but always, or targets the wearer (`target_self`) with a `target` condition: this port holds none of
    /// that for the wearer.
    pub(crate) fn decode_counter(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let present = |key: &str| fields.get(key).is_some_and(|value| !value.is_null());
        if present("holder") { return Ok(None); }
        if fields.get("direct_damage").and_then(Value::as_str).is_some_and(|mode| mode != "always") { return Ok(None); }
        let source = match fields.get("damage_source").filter(|value| !value.is_null()) {
            None => crate::protection::SourceCondition::Any,
            Some(value) => match crate::protection::SourceCondition::decode(value)? { Some(source) => source, None => return Ok(None) },
        };
        let whom = if fields.get("target_self").and_then(Value::as_bool).unwrap_or(false) { Whom::Wearer } else { Whom::Attacker };
        // An effect on the wearer is not filtered by the target condition (the original repurposes `holder` for the attacker there).
        if whom == Whom::Wearer && present("target") { return Ok(None); }
        Ok(Self::decode_as(fields, true)?.map(|module| Self { whom, source, ..module }))
    }

    /// MobEffectModule.ArmorAttack: armor that puts the effect on the mob its wearer hits. A melee hit by the wearer is direct damage of the
    /// `player_attack` type; the module stays unbound when a damage source condition needs state this port lacks. A condition no melee hit
    /// can meet (an arrow's tag, `direct_damage` false) still binds: such a module never lands here, as in the original for a melee hit.
    pub(crate) fn decode_armor_attack(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let source = match fields.get("damage_source").filter(|value| !value.is_null()) {
            None => crate::protection::SourceCondition::Any,
            Some(value) => match crate::protection::SourceCondition::decode(value)? { Some(source) => source, None => return Ok(None) },
        };
        let direct = match fields.get("direct_damage").and_then(Value::as_str) {
            None | Some("always") => None,
            Some("true") => Some(true),
            Some("false") => Some(false),
            Some(other) => return Err(format!("unknown direct_damage {other}")),
        };
        Ok(Self::decode_as(fields, false)?.map(|module| Self { source, direct, ..module }))
    }

    /// The deprecated `tconstruct:mob_effect`: a weapon effect and an armor counter in one, with a chance of 25% a level unless given. The
    /// counter half only ever reaches armor. `None` when a condition names state this port does not hold.
    pub(crate) fn decode_legacy(fields: &Map<String, Value>) -> Result<Option<(Self, Self)>, String> {
        let mut fields = fields.clone();
        if fields.get("chance").map_or(true, Value::is_null) { fields.insert("chance".into(), serde_json::json!({"each_level": 0.25})); }
        let Some(weapon) = Self::decode_as(&fields, false)? else { return Ok(None); };
        let Some(counter) = Self::decode_as(&fields, true)? else { return Ok(None); };
        Ok(Some((weapon, counter)))
    }

    fn decode_as(fields: &Map<String, Value>, counter: bool) -> Result<Option<Self>, String> {
        let effect = Identifier::parse(fields.get("effect").and_then(Value::as_str).ok_or("effect missing")?).map_err(|e| e.to_string())?;
        let level = RandomLeveling::decode(fields, "level")?;
        let time = RandomLeveling::decode(fields, "time")?;
        let Some(target) = TargetCondition::decode_for(if counter { "attacker" } else { "target" }, fields.get("target"))? else { return Ok(None); };
        let Some(holder) = TargetCondition::decode_for(if counter { "" } else { "attacker" }, fields.get("holder"))? else { return Ok(None); };
        // LevelingValue.ONE unless a chance is given: flat 0 + 1 per level is not the default, the default is a flat 1.
        let (chance_flat, chance_each) = match fields.get("chance").filter(|value| !value.is_null()) {
            None => if counter { (0.0, 0.15) } else { (1.0, 0.0) },
            Some(Value::Object(chance)) => {
                let float = |name: &str| chance.get(name).and_then(Value::as_f64).map_or(0.0, |value| value as f32);
                (float("flat"), float("each_level"))
            }
            Some(_) => return Err("chance must be an object".into()),
        };
        let before_melee = fields.get("apply_before_melee").and_then(Value::as_bool).unwrap_or(false);
        Ok(Some(Self { effect, level, time, target, holder, chance_flat, chance_each, before_melee, whom: Whom::Attacker, source: crate::protection::SourceCondition::Any, direct: None }))
    }
    /// Whom the effect goes on: the mob that hit the wearer, or the wearer.
    pub(crate) fn whom(&self) -> Whom { self.whom }
    /// Whether the effect applies before the hit's damage lands instead of after.
    pub(crate) fn before_melee(&self) -> bool { self.before_melee }

    /// MobEffectModule.ArmorAttack.onDamageDealt for a melee hit by the wearer: the effect to put on the mob hit, or None. `source` is the
    /// hit's damage; `in_tag` answers the entity tags and states as for `grant`.
    pub(crate) fn grant_armor_attack(&self, entry: &Entry, condition: bool, source: &crate::protection::DamageSource, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<Grant> {
        if self.direct == Some(false) || !self.source.matches(source) { return None; }
        self.grant(entry, condition, in_tag, rolls)
    }

    /// MobEffectModule.ArmorCounter.onAttacked for a hit the wearer took: the effect to put on the mob that dealt it (or on the
    /// wearer, for `target_self`), or None. `source` is the hit's damage, which the original matches before the chance roll.
    pub(crate) fn grant_counter(&self, entry: &Entry, condition: bool, source: &crate::protection::DamageSource, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<Grant> {
        if !self.source.matches(source) { return None; }
        self.grant(entry, condition, in_tag, rolls)
    }

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

/// BooleanPredicate: which values of a boolean a module accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Accepts { Always, True, False }

impl Accepts {
    fn decode(fields: &Map<String, Value>, key: &str, default: Self) -> Result<Self, String> {
        match fields.get(key).filter(|value| !value.is_null()) {
            None => Ok(default),
            Some(Value::String(mode)) => match mode.as_str() {
                "always" => Ok(Self::Always), "true" => Ok(Self::True), "false" => Ok(Self::False),
                other => Err(format!("unknown {key} {other}")),
            },
            Some(_) => Err(format!("{key} must be a string")),
        }
    }
    /// BooleanPredicate.test: `ALWAYS` or a value equal to what the predicate wants.
    fn test(self, value: bool) -> bool { self == Self::Always || value == (self == Self::True) }
}

/// A use of a tool that MobEffectModule.ToolUsage hears, with the two booleans the original reads off it: whether it counts as an area
/// (`is_aoe`) and whether a projectile caused it (`is_projectile`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Usage {
    /// afterMeleeHit: the swing's own target (not an extra attack of an area tool).
    MeleeHit,
    /// onProjectileLaunch: an arrow of a shot; only the primary arrow is not an area (a multishot's others are).
    Launch { primary: bool },
    /// afterShearEntity: the mob sheared; `target` is false for the extra mobs an area tool shears.
    Shear { target: bool },
    /// afterBlockBreak: a block the tool broke (an area tool's extra blocks are not ported, so the one block is never an area).
    BlockBreak,
}

impl Usage {
    /// (`is_aoe`, `is_projectile`) as the original passes them to the module's predicates.
    fn flags(self) -> (bool, bool) {
        match self {
            Self::MeleeHit => (false, false),
            Self::Launch { primary } => (!primary, false),
            Self::Shear { target } => (!target, false),
            Self::BlockBreak => (false, false),
        }
    }
}

/// MobEffectModule.ToolUsage: an effect the tool puts on the entity that uses it, with a chance that grows with the level. The hooks the
/// module names (all of them when it names none) say which uses it hears. Not ported: the halves for harvesting a plant, a sling's
/// launch and a projectile hitting something (this port has no event for them), and the full-charge check of a melee swing (the hit
/// event does not carry the attack strength).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ToolUsage {
    effect: Identifier, level: RandomLeveling, time: RandomLeveling, chance_flat: f32, chance_each: f32,
    is_aoe: Accepts, is_projectile: Accepts, melee: bool, launch: bool, shear: bool, block_break: bool,
}

impl ToolUsage {
    /// `None` when the module would never land here: it names a `target` condition on the holder (state this port does not hold for the
    /// holder), or the hooks and predicates it names only ever match the uses this port cannot hear.
    pub(crate) fn decode(fields: &Map<String, Value>, hooks: &[Identifier]) -> Result<Option<Self>, String> {
        let effect = Identifier::parse(fields.get("effect").and_then(Value::as_str).ok_or("effect missing")?).map_err(|e| e.to_string())?;
        let level = RandomLeveling::decode(fields, "level")?;
        let time = RandomLeveling::decode(fields, "time")?;
        if fields.get("target").is_some_and(|value| !value.is_null()) { return Ok(None); }
        let (chance_flat, chance_each) = match fields.get("chance").filter(|value| !value.is_null()) {
            None => (1.0, 0.0),
            Some(Value::Object(chance)) => {
                let float = |name: &str| chance.get(name).and_then(Value::as_f64).map_or(0.0, |value| value as f32);
                (float("flat"), float("each_level"))
            }
            Some(_) => return Err("chance must be an object".into()),
        };
        let is_aoe = Accepts::decode(fields, "is_aoe", Accepts::False)?;
        let is_projectile = Accepts::decode(fields, "is_projectile", Accepts::False)?;
        let names = |wanted: &[&str]| hooks.is_empty() || hooks.iter().any(|hook| wanted.contains(&hook.to_string().as_str()));
        let module = Self {
            effect, level, time, chance_flat, chance_each, is_aoe, is_projectile,
            melee: names(&["tconstruct:melee_hit"]),
            launch: names(&["tconstruct:projectile_launch", "tconstruct:projectile_shot", "tconstruct:projectile_thrown"]),
            shear: names(&["tconstruct:shear_entity"]),
            block_break: names(&["tconstruct:block_break"]),
        };
        let lands = |heard: bool, usage: Usage| heard && module.accepts(usage);
        let any = lands(module.melee, Usage::MeleeHit) || lands(module.launch, Usage::Launch { primary: true }) || lands(module.shear, Usage::Shear { target: true }) || lands(module.block_break, Usage::BlockBreak);
        Ok(any.then_some(module))
    }

    fn accepts(&self, usage: Usage) -> bool {
        let (aoe, projectile) = usage.flags();
        self.is_aoe.test(aoe) && self.is_projectile.test(projectile)
    }

    /// MobEffectModule.ToolUsage for `usage`: the effect to put on the holder, or None. `rolls` gives uniform values in [0, 1): the chance
    /// roll, the level's, then the duration's.
    pub(crate) fn grant(&self, entry: &Entry, condition: bool, usage: Usage, rolls: &mut dyn FnMut() -> f32) -> Option<Grant> {
        let heard = match usage { Usage::MeleeHit => self.melee, Usage::Launch { .. } => self.launch, Usage::Shear { .. } => self.shear, Usage::BlockBreak => self.block_break };
        if !heard || !condition || !self.accepts(usage) { return None; }
        let scaled = entry.effective_level();
        let chance = self.chance_flat + self.chance_each * scaled;
        if !(chance > 0.0 && (chance >= 1.0 || rolls() < chance)) { return None; }
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
    fn an_armor_counter_defaults_its_chance_and_stays_unbound_for_what_it_cannot_hold() {
        let decay = WeaponEffect::decode_counter(json!({"effect":"minecraft:wither","level":{"flat":2.0},"time":{"flat":120.0},"chance":{"each_level":0.5}}).as_object().unwrap()).unwrap().unwrap();
        assert_eq!(decay.grant(&entry(1), true, &all_tags, &mut || 0.49), Some(Grant { effect: Identifier::parse("minecraft:wither").unwrap(), amplifier: 1, duration: 120 }));
        assert!(decay.grant(&entry(1), true, &all_tags, &mut || 0.51).is_none());
        let plain = WeaponEffect::decode_counter(json!({"effect":"minecraft:glowing","level":{"flat":1.0},"time":{"flat":200.0}}).as_object().unwrap()).unwrap().unwrap();
        assert!(plain.grant(&entry(2), true, &all_tags, &mut || 0.29).is_some(), "15% a level: 30% at level 2");
        assert!(plain.grant(&entry(2), true, &all_tags, &mut || 0.31).is_none());
        let counter = |value: Value| WeaponEffect::decode_counter(value.as_object().unwrap()).unwrap();
        let revenge = counter(json!({"effect":"minecraft:strength","level":{"flat":1.0},"time":{"flat":200.0},"target_self":true,"chance":{"flat":1.0}})).unwrap();
        assert_eq!(revenge.whom(), Whom::Wearer, "an effect on the wearer is bound");
        assert!(counter(json!({"effect":"minecraft:strength","level":{"flat":1.0},"time":{"flat":200.0},"target_self":true,"target":"mantle:on_fire"})).is_none(), "a target condition on an effect for the wearer stays unbound");
        assert!(counter(json!({"effect":"minecraft:strength","level":{"flat":1.0},"time":{"flat":200.0},"damage_source":{"type":"mantle:attacker"}})).is_none(), "a damage source condition this port does not hold stays unbound");
        assert!(counter(json!({"effect":"minecraft:strength","level":{"flat":1.0},"time":{"flat":200.0},"damage_source":{"type":"mantle:in_biome","biome":"minecraft:plains"}})).is_none(), "an unknown damage source predicate stays unbound");
        assert!(counter(json!({"effect":"minecraft:strength","level":{"flat":1.0},"time":{"flat":200.0},"holder":"mantle:sprinting"})).is_none(), "a wearer condition stays unbound");
    }

    #[test]
    fn a_bare_mantle_any_is_the_condition_that_asks_nothing() {
        for role in ["attacker", "target"] {
            assert_eq!(TargetCondition::decode_for(role, Some(&json!("mantle:any"))).unwrap(), Some(TargetCondition::Any), "{role}");
        }
        assert_eq!(TargetCondition::decode_for("target", Some(&json!({"type":"mantle:any"}))).unwrap(), Some(TargetCondition::Any), "the object form agrees");
    }

    #[test]
    fn the_shipped_revenge_counter_binds_and_lands_on_the_wearer() {
        // revenge: strength for 200 ticks a level on the wearer, whenever the entity that caused the hit is any entity at all.
        let revenge = WeaponEffect::decode_counter(json!({"chance":{"flat":1.0},
            "damage_source":{"type":"mantle:attacker","entity_type":"mantle:any","which":"causing"},
            "direct_damage":"always","durability_usage":0,"effect":"minecraft:strength","level":{"flat":1.0},
            "target_self":true,"time":{"per_level":200.0}}).as_object().unwrap()).unwrap().unwrap();
        assert_eq!(revenge.whom(), Whom::Wearer);
        let tags = melee_tags();
        let mob_attack = Identifier::parse("minecraft:mob_attack").unwrap();
        let hit = source(&mob_attack, &tags, &all_tags);
        assert_eq!(revenge.grant_counter(&entry(1), true, &hit, &all_tags, &mut || 0.0), Some(Grant { effect: Identifier::parse("minecraft:strength").unwrap(), amplifier: 0, duration: 200 }));
        assert_eq!(revenge.grant_counter(&entry(2), true, &hit, &all_tags, &mut || 0.0).unwrap().duration, 400, "200 ticks a level");
        // A counter whose source condition asks about the causing entity lands only when that entity passes it.
        let picky = WeaponEffect::decode_counter(json!({"effect":"minecraft:strength","level":{"flat":1.0},"time":{"flat":200.0},
            "damage_source":{"type":"mantle:attacker","entity_type":"mantle:on_fire","which":"causing"}}).as_object().unwrap()).unwrap().unwrap();
        let state_only = |tag: &Identifier| tag.namespace() == crate::melee_damage::STATE_NAMESPACE;
        let burning = source(&mob_attack, &tags, &state_only);
        assert!(picky.grant_counter(&entry(1), true, &burning, &all_tags, &mut || 0.0).is_some(), "the causing mob was on fire");
        let calm = source(&mob_attack, &tags, &no_tags);
        assert!(picky.grant_counter(&entry(1), true, &calm, &all_tags, &mut || 0.0).is_none(), "the causing mob was not on fire");
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

    fn melee_tags() -> crate::damage_tags::DamageTags {
        let stack = pluto_lib::JsonResourceStack { id: Identifier::parse("tconstruct:protection/melee").unwrap(), layers: vec![Ok(json!({"values":["minecraft:player_attack"]}))] };
        crate::damage_tags::DamageTags::load(&[stack]).0
    }
    fn source<'a>(damage_type: &'a Identifier, tags: &'a crate::damage_tags::DamageTags, attacker: &'a dyn Fn(&Identifier) -> bool) -> crate::protection::DamageSource<'a> {
        crate::protection::DamageSource { damage_type, indirect: false, tags, attacker_in_tag: attacker, entities: Default::default() }
    }

    #[test]
    fn an_armor_attack_lands_only_for_the_damage_source_it_names() {
        let tags = melee_tags();
        let player_attack = Identifier::parse("minecraft:player_attack").unwrap();
        let (hit, none) = (source(&player_attack, &tags, &no_tags), source(&player_attack, &tags, &no_tags));
        let arrow = Identifier::parse("minecraft:arrow").unwrap();
        let shot = source(&arrow, &tags, &no_tags);
        // withered: wither II-less (level 1) for six seconds on a melee hit of the wearer.
        let withered = WeaponEffect::decode_armor_attack(json!({"effect":"minecraft:wither","level":{"flat":1.0},"time":{"flat":120.0},
            "damage_source":{"type":"mantle:tag","tag":"tconstruct:protection/melee"}}).as_object().unwrap()).unwrap().unwrap();
        assert_eq!(withered.grant_armor_attack(&entry(1), true, &hit, &all_tags, &mut || 0.0), Some(Grant { effect: Identifier::parse("minecraft:wither").unwrap(), amplifier: 0, duration: 120 }));
        assert!(withered.grant_armor_attack(&entry(1), true, &shot, &all_tags, &mut || 0.0).is_none(), "an arrow is not a melee source");
        assert!(withered.grant_armor_attack(&entry(1), false, &none, &all_tags, &mut || 0.0).is_none(), "the modifier condition holds first");
        // slow bones: the same wearer's melee hit never meets an is_projectile source.
        let bones = WeaponEffect::decode_armor_attack(json!({"effect":"minecraft:slowness","level":{"per_level":2.0},"time":{"flat":300.0},
            "damage_source":{"type":"mantle:tag","tag":"minecraft:is_projectile"}}).as_object().unwrap()).unwrap().unwrap();
        assert!(bones.grant_armor_attack(&entry(1), true, &hit, &all_tags, &mut || 0.0).is_none());
        let indirect_only = WeaponEffect::decode_armor_attack(json!({"effect":"minecraft:slowness","level":{"flat":1.0},"time":{"flat":20.0},"direct_damage":"false"}).as_object().unwrap()).unwrap().unwrap();
        assert!(indirect_only.grant_armor_attack(&entry(1), true, &hit, &all_tags, &mut || 0.0).is_none(), "a melee hit is direct damage");
        assert!(WeaponEffect::decode_armor_attack(json!({"effect":"minecraft:slowness","level":{"flat":1.0},"time":{"flat":20.0},"damage_source":{"type":"mantle:attacker"}}).as_object().unwrap()).unwrap().is_none(), "an unknown source predicate stays unbound");
    }

    #[test]
    fn the_legacy_mob_effect_is_a_weapon_effect_and_a_counter_with_a_quarter_chance_a_level() {
        // sticky: slowness, level 0.5 a level, 20 ticks plus up to 10 a level.
        let (weapon, counter) = WeaponEffect::decode_legacy(json!({"effect":"minecraft:slowness","level":{"per_level":0.5},"time":{"flat":20.0,"random":10.0}}).as_object().unwrap()).unwrap().unwrap();
        assert!(weapon.grant(&entry(2), true, &all_tags, &mut || 0.49).is_some() && weapon.grant(&entry(2), true, &all_tags, &mut || 0.51).is_none(), "25% a level: 50% at level 2");
        assert_eq!(counter.whom(), Whom::Attacker);
        assert!(counter.grant(&entry(2), true, &all_tags, &mut || 0.49).is_some() && counter.grant(&entry(2), true, &all_tags, &mut || 0.51).is_none(), "the counter shares the chance");
        let grant = weapon.grant(&entry(2), true, &all_tags, &mut || 0.4).unwrap();
        assert_eq!((grant.amplifier, grant.duration), (0, 28), "level 1 from 0.5 a level, 20 + 0.4 of 10 for each of two levels");
        let (given, _) = WeaponEffect::decode_legacy(json!({"effect":"minecraft:slowness","level":{"flat":1.0},"time":{"flat":20.0},"chance":{"flat":1.0}}).as_object().unwrap()).unwrap().unwrap();
        assert!(given.grant(&entry(1), true, &all_tags, &mut || 0.99).is_some(), "a chance given replaces the default");
    }

    fn usage(value: Value, hooks: &[&str]) -> Option<ToolUsage> {
        let hooks: Vec<Identifier> = hooks.iter().map(|hook| Identifier::parse(hook).unwrap()).collect();
        ToolUsage::decode(value.as_object().unwrap(), &hooks).unwrap()
    }

    #[test]
    fn a_tool_usage_effect_lands_only_for_the_uses_its_hooks_name() {
        // decay: wither on the holder, a quarter of the time, from a swing or a shot but not from shearing.
        let decay = usage(json!({"effect":"minecraft:wither","chance":{"flat":0.25},"level":{"per_level":1.0},"time":{"flat":120.0}}),
            &["tconstruct:melee_hit", "tconstruct:monster_melee_hit", "tconstruct:projectile_launch", "tconstruct:projectile_shot", "tconstruct:projectile_thrown", "tconstruct:sling_launch"]).unwrap();
        let wither = |amplifier| Some(Grant { effect: Identifier::parse("minecraft:wither").unwrap(), amplifier, duration: 120 });
        assert_eq!(decay.grant(&entry(2), true, Usage::MeleeHit, &mut || 0.2), wither(1));
        assert_eq!(decay.grant(&entry(1), true, Usage::Launch { primary: true }, &mut || 0.2), wither(0));
        assert!(decay.grant(&entry(1), true, Usage::MeleeHit, &mut || 0.3).is_none(), "a roll over 25% misses");
        assert!(decay.grant(&entry(1), true, Usage::Shear { target: true }, &mut || 0.0).is_none(), "decay does not hear shearing");
        assert!(decay.grant(&entry(1), true, Usage::Launch { primary: false }, &mut || 0.0).is_none(), "a multishot's extra arrow counts as an area, which is not accepted");
        assert!(decay.grant(&entry(1), false, Usage::MeleeHit, &mut || 0.0).is_none(), "the modifier condition holds first");
    }

    #[test]
    fn a_tool_usage_effect_with_no_hooks_hears_every_use_and_the_predicates_can_widen_it() {
        // magnetic: every hook by default, and the chance defaults to certain.
        let magnetic = usage(json!({"effect":"tconstruct:magnetic","level":{"flat":1.0},"time":{"flat":40.0}}), &[]).unwrap();
        for use_of in [Usage::MeleeHit, Usage::Launch { primary: true }, Usage::Shear { target: true }, Usage::BlockBreak] {
            assert_eq!(magnetic.grant(&entry(1), true, use_of, &mut || 0.99).map(|grant| grant.duration), Some(40), "{use_of:?}");
        }
        let area = json!({"effect":"minecraft:haste","level":{"flat":1.0},"time":{"flat":40.0},"is_aoe":"true"});
        assert!(usage(area, &[]).is_none(), "an effect for the extra targets of an area tool never lands: this port reports only the primary one");
        let anywhere = usage(json!({"effect":"minecraft:haste","level":{"flat":1.0},"time":{"flat":40.0},"is_aoe":"always"}), &["tconstruct:shear_entity"]).unwrap();
        assert!(anywhere.grant(&entry(1), true, Usage::Shear { target: true }, &mut || 0.0).is_some() && anywhere.grant(&entry(1), true, Usage::MeleeHit, &mut || 0.0).is_none(), "only the hooks it names");
    }

    #[test]
    fn a_tool_usage_effect_that_could_never_land_here_stays_unbound() {
        let base = json!({"effect":"minecraft:haste","level":{"flat":1.0},"time":{"flat":40.0}});
        let breaking = usage(base.clone(), &["tconstruct:block_break"]).expect("a block the tool broke is heard now");
        assert!(breaking.grant(&entry(1), true, Usage::BlockBreak, &mut || 0.0).is_some() && breaking.grant(&entry(1), true, Usage::MeleeHit, &mut || 0.0).is_none(), "only the block-break half");
        assert!(usage(base.clone(), &["tconstruct:sling_launch", "tconstruct:plant_harvest"]).is_none());
        let mut projectile = base.clone();
        projectile["is_projectile"] = json!("true");
        assert!(usage(projectile, &[]).is_none(), "an effect for the holder of a projectile that hit has no use here");
        let mut targeted = base.clone();
        targeted["target"] = json!("mantle:on_fire");
        assert!(usage(targeted, &[]).is_none(), "a condition on the holder stays unbound");
        assert!(ToolUsage::decode(json!({"effect":"minecraft:haste","level":{"flat":1.0}}).as_object().unwrap(), &[]).is_err(), "the time is required");
    }
}
